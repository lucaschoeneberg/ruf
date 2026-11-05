use super::*;
use rusqlite::{Connection, OptionalExtension, params};
use std::{collections::HashMap, path::Path};

pub(super) struct HistoryDb {
    connection: Connection,
    // Process-local native tokens are never persisted or reused as history IDs.
    observations: HashMap<u64, String>,
}
impl HistoryDb {
    pub(super) fn open(path: &Path) -> Result<Self, PhoneError> {
        std::fs::create_dir_all(path.parent().ok_or_else(storage_error)?)
            .map_err(|_| storage_error())?;
        Self::open_connection(Connection::open(path).map_err(|_| storage_error())?)
    }
    fn open_connection(mut connection: Connection) -> Result<Self, PhoneError> {
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| storage_error())?;
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(|_| storage_error())?;
        if integrity != "ok" {
            return Err(storage_error());
        }
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|_| storage_error())?;
        if version == 0 {
            let count: u32 = connection
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| storage_error())?;
            if count != 0 {
                return Err(storage_error());
            }
            let transaction = connection.transaction().map_err(|_| storage_error())?;
            transaction.execute_batch("CREATE TABLE calls (id TEXT PRIMARY KEY NOT NULL, started_at INTEGER NOT NULL, direction TEXT NOT NULL, outcome TEXT NOT NULL, facts TEXT NOT NULL); CREATE INDEX calls_newest ON calls(started_at DESC,id DESC); CREATE INDEX calls_direction ON calls(direction,started_at DESC,id DESC); CREATE INDEX calls_outcome ON calls(outcome,started_at DESC,id DESC); PRAGMA user_version=1;").map_err(|_| storage_error())?;
            transaction.commit().map_err(|_| storage_error())?;
        } else if version != 1 {
            return Err(storage_error());
        }
        let mut db = Self {
            connection,
            observations: HashMap::new(),
        };
        // Validate every stored payload before recovery writes. Never replace an
        // unknown schema or malformed database with a deceptively empty history.
        let rows = db.entries(HistoryFilter::All)?;
        let transaction = db.connection.transaction().map_err(|_| storage_error())?;
        for mut row in rows
            .into_iter()
            .filter(|r| r.outcome == CallOutcome::InProgress)
        {
            row.outcome = CallOutcome::Interrupted;
            row.ended_at_ms = None;
            row.duration_ms = None;
            Self::put(&transaction, &row)?;
        }
        transaction.commit().map_err(|_| storage_error())?;
        Ok(db)
    }
    fn put(connection: &Connection, entry: &HistoryEntry) -> Result<(), PhoneError> {
        let mut facts = serde_json::to_value(entry).map_err(|_| storage_error())?;
        let object = facts.as_object_mut().ok_or_else(storage_error)?;
        object.remove("contact");
        object.remove("callbackAccountId");
        let direction = match entry.direction {
            CallDirection::Incoming => "incoming",
            CallDirection::Outgoing => "outgoing",
        };
        let outcome = serde_json::to_value(entry.outcome).map_err(|_| storage_error())?;
        let started = i64::try_from(entry.started_at_ms).map_err(|_| storage_error())?;
        connection.execute("INSERT INTO calls(id,started_at,direction,outcome,facts) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET outcome=excluded.outcome,facts=excluded.facts", params![entry.id, started, direction, outcome.as_str().ok_or_else(storage_error)?, facts.to_string()]).map_err(|_| storage_error())?;
        Ok(())
    }
    pub(super) fn get(&self, id: &str) -> Result<HistoryEntry, PhoneError> {
        let facts: Option<String> = self
            .connection
            .query_row("SELECT facts FROM calls WHERE id=?1", [id], |r| r.get(0))
            .optional()
            .map_err(|_| storage_error())?;
        let entry: HistoryEntry =
            serde_json::from_str(&facts.ok_or_else(|| {
                PhoneError::new(ErrorCode::InvalidInput, "Unknown history entry.")
            })?)
            .map_err(|_| storage_error())?;
        Ok(entry)
    }
    pub(super) fn observe(&mut self, event: ObservedCallEvent) -> Result<bool, PhoneError> {
        let row = match event.kind {
            ObservedCallKind::Started {
                account,
                direction,
                number,
                remote_name,
            } => {
                if self.observations.contains_key(&event.observation_id) {
                    return Ok(false);
                }
                let mut bytes = [0_u8; 16];
                getrandom::fill(&mut bytes).map_err(|_| storage_error())?;
                let id: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                HistoryEntry {
                    id,
                    account_name: account
                        .as_ref()
                        .map_or_else(String::new, |a| a.name.clone()),
                    account_registrar: account.as_ref().map(|a| a.registrar.clone()),
                    account_username: account.map(|a| a.username),
                    direction,
                    number,
                    remote_name,
                    started_at_ms: event.at_ms,
                    answered_at_ms: None,
                    ended_at_ms: None,
                    duration_ms: None,
                    outcome: CallOutcome::InProgress,
                    sip_status: None,
                    contact: None,
                    callback_account_id: None,
                }
            }
            kind => {
                let Some(id) = self.observations.get(&event.observation_id) else {
                    return Ok(false);
                };
                let mut entry = match self.get(id) {
                    Ok(entry) => entry,
                    Err(error) if error.code == ErrorCode::InvalidInput => return Ok(false),
                    Err(error) => return Err(error),
                };
                if entry.outcome != CallOutcome::InProgress {
                    return Ok(false);
                }
                match kind {
                    ObservedCallKind::Connected => {
                        if entry.answered_at_ms.is_some() {
                            return Ok(false);
                        }
                        entry.answered_at_ms = Some(event.at_ms);
                    }
                    ObservedCallKind::Finished {
                        outcome,
                        sip_status,
                        duration_ms,
                    } => {
                        if outcome == CallOutcome::InProgress {
                            return Err(storage_error());
                        }
                        entry.outcome = outcome;
                        entry.ended_at_ms = Some(event.at_ms);
                        entry.duration_ms = entry.answered_at_ms.and(duration_ms);
                        entry.sip_status = sip_status.filter(|status| (100..=699).contains(status));
                    }
                    ObservedCallKind::Started { .. } => unreachable!(),
                }
                entry
            }
        };
        let transaction = self.connection.transaction().map_err(|_| storage_error())?;
        Self::put(&transaction, &row)?;
        transaction.commit().map_err(|_| storage_error())?;
        self.observations.insert(event.observation_id, row.id);
        Ok(true)
    }
    fn predicate(filter: HistoryFilter) -> &'static str {
        match filter {
            HistoryFilter::All => "",
            HistoryFilter::Missed => " WHERE outcome='missed' AND direction='incoming'",
            HistoryFilter::Incoming => " WHERE direction='incoming'",
            HistoryFilter::Outgoing => " WHERE direction='outgoing'",
        }
    }
    pub(super) fn entries(&self, filter: HistoryFilter) -> Result<Vec<HistoryEntry>, PhoneError> {
        let sql = format!(
            "SELECT facts FROM calls{} ORDER BY started_at DESC,id DESC",
            Self::predicate(filter)
        );
        self.read_rows(&sql, [])
    }
    fn read_rows(
        &self,
        sql: &str,
        parameters: impl rusqlite::Params,
    ) -> Result<Vec<HistoryEntry>, PhoneError> {
        let mut statement = self.connection.prepare(sql).map_err(|_| storage_error())?;
        let facts = statement
            .query_map(parameters, |row| row.get::<_, String>(0))
            .map_err(|_| storage_error())?;
        facts
            .map(|facts| {
                serde_json::from_str(&facts.map_err(|_| storage_error())?)
                    .map_err(|_| storage_error())
            })
            .collect()
    }
    pub(super) fn page(
        &self,
        filter: HistoryFilter,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<HistoryEntry>, u32), PhoneError> {
        let predicate = Self::predicate(filter);
        let total = self
            .connection
            .query_row(&format!("SELECT count(*) FROM calls{predicate}"), [], |r| {
                r.get::<_, u32>(0)
            })
            .map_err(|_| storage_error())?;
        let sql = format!(
            "SELECT facts FROM calls{predicate} ORDER BY started_at DESC,id DESC LIMIT ?1 OFFSET ?2"
        );
        Ok((self.read_rows(&sql, params![limit, offset])?, total))
    }
    pub(super) fn delete(&mut self, id: &str) -> Result<bool, PhoneError> {
        let entry = self.get(id)?;
        if entry.outcome == CallOutcome::InProgress {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "An ongoing call cannot be deleted.",
            ));
        }
        let transaction = self.connection.transaction().map_err(|_| storage_error())?;
        let changed = transaction
            .execute("DELETE FROM calls WHERE id=?1", [id])
            .map_err(|_| storage_error())?
            > 0;
        transaction.commit().map_err(|_| storage_error())?;
        Ok(changed)
    }
    pub(super) fn configure(
        &mut self,
        previous: &HistoryPreferences,
        next: &HistoryPreferences,
        confirmed: bool,
        now_ms: u64,
        persist: impl FnOnce() -> Result<(), PhoneError>,
    ) -> Result<bool, PhoneError> {
        next.validate()?;
        let cutoff = retention_cutoff(next, now_ms)
            .map(i64::try_from)
            .transpose()
            .map_err(|_| storage_error())?;
        let shorter = next
            .retention_days
            .is_some_and(|days| previous.retention_days.is_none_or(|old| days < old));
        let transaction = self.connection.transaction().map_err(|_| storage_error())?;
        let changed = if let Some(cutoff) = cutoff {
            let expired: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM calls WHERE outcome!='inProgress' AND started_at < ?1)", [cutoff], |row| row.get(0)).map_err(|_| storage_error())?;
            if expired && shorter && !confirmed {
                return Err(PhoneError::new(
                    ErrorCode::InvalidInput,
                    "Confirm deletion of existing history before shortening retention.",
                ));
            }
            transaction
                .execute(
                    "DELETE FROM calls WHERE outcome!='inProgress' AND started_at < ?1",
                    [cutoff],
                )
                .map_err(|_| storage_error())?
                > 0
        } else {
            false
        };
        // Keep the SQLite deletion uncommitted until metadata replacement succeeds.
        persist()?;
        transaction.commit().map_err(|_| storage_error())?;
        Ok(changed)
    }

    pub(super) fn prune_before(&mut self, cutoff_ms: u64) -> Result<bool, PhoneError> {
        let cutoff = i64::try_from(cutoff_ms).map_err(|_| storage_error())?;
        let transaction = self.connection.transaction().map_err(|_| storage_error())?;
        let changed = transaction
            .execute(
                "DELETE FROM calls WHERE outcome!='inProgress' AND started_at < ?1",
                [cutoff],
            )
            .map_err(|_| storage_error())?
            > 0;
        transaction.commit().map_err(|_| storage_error())?;
        Ok(changed)
    }

    pub(super) fn clear(&mut self) -> Result<bool, PhoneError> {
        let transaction = self.connection.transaction().map_err(|_| storage_error())?;
        let changed = transaction
            .execute("DELETE FROM calls WHERE outcome!='inProgress'", [])
            .map_err(|_| storage_error())?
            > 0;
        transaction.commit().map_err(|_| storage_error())?;
        Ok(changed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use phone_core::{AccountSnapshot, RegistrationState};

    fn account() -> AccountSnapshot {
        AccountSnapshot {
            id: 1,
            name: "Synthetic".into(),
            registrar: "pbx.test".into(),
            username: "100".into(),
            registration: RegistrationState::Registered,
            last_error: None,
            removing: false,
            enabled: true,
            enabled_changing: false,
            registered_at_ms: None,
        }
    }
    fn started(id: u64) -> ObservedCallEvent {
        ObservedCallEvent {
            observation_id: id,
            at_ms: 100,
            kind: ObservedCallKind::Started {
                account: Some(account()),
                direction: CallDirection::Outgoing,
                number: Some("123".into()),
                remote_name: Some("SIP name".into()),
            },
        }
    }
    fn event(id: u64, at_ms: u64, kind: ObservedCallKind) -> ObservedCallEvent {
        ObservedCallEvent {
            observation_id: id,
            at_ms,
            kind,
        }
    }
    fn db() -> HistoryDb {
        HistoryDb::open_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn collection_disabled_at_start_never_writes_later_and_recorded_calls_can_finish() {
        let mut db = db();
        let mut policy = RecordingPolicy::new(false);
        let start = started(1);
        if policy.record(&start) {
            db.observe(start).unwrap();
        }
        policy.enabled = true;
        for event in [
            event(1, 120, ObservedCallKind::Connected),
            event(
                1,
                130,
                ObservedCallKind::Finished {
                    outcome: CallOutcome::Completed,
                    sip_status: None,
                    duration_ms: Some(10),
                },
            ),
        ] {
            if policy.record(&event) {
                db.observe(event).unwrap();
            }
        }
        assert!(db.entries(HistoryFilter::All).unwrap().is_empty());
        let start = started(2);
        if policy.record(&start) {
            db.observe(start).unwrap();
        }
        policy.enabled = false;
        let finish = event(
            2,
            140,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Failed,
                sip_status: None,
                duration_ms: None,
            },
        );
        if policy.record(&finish) {
            db.observe(finish).unwrap();
        }
        let rows = db.entries(HistoryFilter::All).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].outcome, CallOutcome::Failed);
        assert_eq!(rows[0].ended_at_ms, Some(140));
    }

    #[test]
    fn shorter_retention_requires_confirmation_and_failed_persistence_rolls_back_deletion() {
        let mut db = db();
        db.observe(started(1)).unwrap();
        db.observe(event(
            1,
            101,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Completed,
                sip_status: None,
                duration_ms: None,
            },
        ))
        .unwrap();
        let previous = HistoryPreferences::default();
        let next = HistoryPreferences {
            retention_days: Some(30),
            ..previous.clone()
        };
        let now = 31 * 24 * 60 * 60 * 1000;
        let error = db
            .configure(&previous, &next, false, now, || {
                panic!("unconfirmed deletion must not persist preferences")
            })
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert_eq!(db.entries(HistoryFilter::All).unwrap().len(), 1);
        assert_eq!(
            db.configure(&previous, &next, true, now, || Err(storage_error()))
                .unwrap_err()
                .code,
            ErrorCode::StorageFailure
        );
        assert_eq!(db.entries(HistoryFilter::All).unwrap().len(), 1);
        assert!(
            db.configure(&previous, &next, true, now, || Ok(()))
                .unwrap()
        );
        assert!(db.entries(HistoryFilter::All).unwrap().is_empty());
    }

    #[test]
    fn retention_uses_exclusive_start_cutoff_and_protects_active_calls() {
        let mut db = db();
        for id in 1..=3 {
            let mut start = started(id);
            start.at_ms = if id == 2 { 200 } else { 100 };
            db.observe(start).unwrap();
            if id != 3 {
                db.observe(event(
                    id,
                    300,
                    ObservedCallKind::Finished {
                        outcome: CallOutcome::Completed,
                        sip_status: None,
                        duration_ms: None,
                    },
                ))
                .unwrap();
            }
        }
        assert!(db.prune_before(200).unwrap());
        let rows = db.entries(HistoryFilter::All).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.started_at_ms == 200));
        assert!(
            rows.iter()
                .any(|row| row.outcome == CallOutcome::InProgress)
        );
        db.connection.execute_batch("PRAGMA query_only=ON").unwrap();
        assert_eq!(
            db.prune_before(201).unwrap_err().code,
            ErrorCode::StorageFailure
        );
        assert_eq!(db.entries(HistoryFilter::All).unwrap().len(), 2);
    }

    #[test]
    fn complete_lifecycle_is_durable_and_duplicate_observations_are_idempotent() {
        let mut db = db();
        db.observe(started(1)).unwrap();
        db.observe(started(1)).unwrap();
        db.observe(event(1, 120, ObservedCallKind::Connected))
            .unwrap();
        db.observe(event(1, 130, ObservedCallKind::Connected))
            .unwrap();
        db.observe(event(
            1,
            90,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Completed,
                sip_status: Some(200),
                duration_ms: Some(45),
            },
        ))
        .unwrap();
        db.observe(event(
            1,
            500,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Failed,
                sip_status: None,
                duration_ms: None,
            },
        ))
        .unwrap();
        let rows = db.entries(HistoryFilter::All).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].answered_at_ms, Some(120));
        assert_eq!(rows[0].ended_at_ms, Some(90)); // Wall clock movement must not alter observed evidence.
        assert_eq!(rows[0].duration_ms, Some(45));
        assert_eq!(rows[0].outcome, CallOutcome::Completed);
        assert!(rows[0].contact.is_none());
        let payload: String = db
            .connection
            .query_row("SELECT facts FROM calls", [], |r| r.get(0))
            .unwrap();
        assert!(!payload.contains("callbackAccountId"));
        assert!(!payload.contains("contact"));
        assert!(!payload.contains("password"));
    }
    #[test]
    fn fast_calls_are_recorded_and_active_records_cannot_be_deleted() {
        let mut db = db();
        db.observe(started(1)).unwrap();
        let active = db.entries(HistoryFilter::All).unwrap().remove(0);
        assert_eq!(db.delete(&active.id).unwrap_err().code, ErrorCode::Busy);
        assert!(!db.clear().unwrap());
        db.observe(event(
            1,
            101,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Busy,
                sip_status: Some(486),
                duration_ms: None,
            },
        ))
        .unwrap();
        assert_eq!(
            db.entries(HistoryFilter::All).unwrap()[0].outcome,
            CallOutcome::Busy
        );
        db.observe(started(2)).unwrap();
        assert!(db.clear().unwrap());
        assert_eq!(db.entries(HistoryFilter::All).unwrap().len(), 1);
        assert_eq!(
            db.delete(&active.id).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    #[test]
    fn restart_marks_only_unfinished_rows_interrupted_without_inventing_end_or_duration() {
        let mut db = db();
        db.observe(started(1)).unwrap();
        db.observe(event(1, 120, ObservedCallKind::Connected))
            .unwrap();
        db.observe(started(2)).unwrap();
        db.observe(event(
            2,
            130,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Failed,
                sip_status: None,
                duration_ms: None,
            },
        ))
        .unwrap();
        let restarted = HistoryDb::open_connection(db.connection).unwrap();
        let rows = restarted.entries(HistoryFilter::All).unwrap();
        assert!(rows.iter().any(|r| r.outcome == CallOutcome::Failed));
        let interrupted = rows
            .iter()
            .find(|r| r.outcome == CallOutcome::Interrupted)
            .unwrap();
        assert_eq!(interrupted.answered_at_ms, Some(120));
        assert_eq!(interrupted.ended_at_ms, None);
        assert_eq!(interrupted.duration_ms, None);
    }
    #[test]
    fn unsupported_schema_and_corruption_are_rejected_without_replacement() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA user_version=2; CREATE TABLE preserved(value); INSERT INTO preserved VALUES(42)").unwrap();
        assert!(HistoryDb::open_connection(connection).is_err());
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE unrelated(value)")
            .unwrap();
        assert!(HistoryDb::open_connection(connection).is_err());
        let mut db = db();
        db.observe(started(1)).unwrap();
        db.connection
            .execute("UPDATE calls SET facts='broken'", [])
            .unwrap();
        assert!(HistoryDb::open_connection(db.connection).is_err());
    }
    #[test]
    fn unknown_events_do_not_create_fake_calls_and_storage_errors_propagate() {
        let mut db = db();
        db.observe(event(1, 100, ObservedCallKind::Connected))
            .unwrap();
        assert!(db.entries(HistoryFilter::All).unwrap().is_empty());
        db.connection.execute_batch("PRAGMA query_only=ON").unwrap();
        assert_eq!(
            db.observe(started(1)).unwrap_err().code,
            ErrorCode::StorageFailure
        );
        db.connection
            .execute_batch("PRAGMA query_only=OFF")
            .unwrap();
        db.observe(started(1)).unwrap();
        assert_eq!(db.entries(HistoryFilter::All).unwrap().len(), 1);
    }
    #[test]
    fn deleting_finished_calls_does_not_turn_duplicate_events_into_storage_failures() {
        let mut db = db();
        db.observe(started(1)).unwrap();
        db.observe(event(
            1,
            101,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Failed,
                sip_status: None,
                duration_ms: None,
            },
        ))
        .unwrap();
        let id = db.entries(HistoryFilter::All).unwrap()[0].id.clone();
        db.delete(&id).unwrap();
        assert!(
            !db.observe(event(
                1,
                102,
                ObservedCallKind::Finished {
                    outcome: CallOutcome::Failed,
                    sip_status: None,
                    duration_ms: None
                }
            ))
            .unwrap()
        );
        assert!(!db.observe(started(1)).unwrap());
        assert!(db.entries(HistoryFilter::All).unwrap().is_empty());
    }
    #[test]
    fn database_pagination_filters_and_orders_newest_first() {
        let mut db = db();
        db.observe(started(1)).unwrap();
        let mut second = started(2);
        second.at_ms = 200;
        if let ObservedCallKind::Started { direction, .. } = &mut second.kind {
            *direction = CallDirection::Incoming;
        }
        db.observe(second).unwrap();
        db.observe(event(
            2,
            201,
            ObservedCallKind::Finished {
                outcome: CallOutcome::Missed,
                sip_status: None,
                duration_ms: None,
            },
        ))
        .unwrap();
        let (rows, total) = db.page(HistoryFilter::All, 0, 1).unwrap();
        assert_eq!(total, 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].started_at_ms, 200);
        let (rows, _) = db.page(HistoryFilter::All, 1, 1).unwrap();
        assert_eq!(rows[0].started_at_ms, 100);
        for filter in [
            HistoryFilter::Incoming,
            HistoryFilter::Outgoing,
            HistoryFilter::Missed,
        ] {
            let (rows, total) = db.page(filter, 0, 50).unwrap();
            assert_eq!(total, 1);
            assert_eq!(rows.len(), 1);
        }
    }
    #[test]
    fn disk_roundtrip_recovers_only_open_calls_and_preserves_corrupt_file_bytes() {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).unwrap();
        let directory = std::env::temp_dir().join(format!(
            "ruf-history-test-{:x}",
            u128::from_ne_bytes(random)
        ));
        let path = directory.join("history.sqlite3");
        {
            let mut db = HistoryDb::open(&path).unwrap();
            db.observe(started(1)).unwrap();
            db.observe(event(1, 110, ObservedCallKind::Connected))
                .unwrap();
            db.observe(event(
                1,
                120,
                ObservedCallKind::Finished {
                    outcome: CallOutcome::Completed,
                    sip_status: Some(200),
                    duration_ms: Some(10),
                },
            ))
            .unwrap();
            db.observe(started(2)).unwrap();
        }
        {
            let mut reopened = HistoryDb::open(&path).unwrap();
            let entries = reopened.entries(HistoryFilter::All).unwrap();
            assert_eq!(entries.len(), 2);
            let completed = entries
                .iter()
                .find(|e| e.outcome == CallOutcome::Completed)
                .unwrap();
            assert_eq!(completed.ended_at_ms, Some(120));
            assert_eq!(completed.duration_ms, Some(10));
            assert!(
                entries
                    .iter()
                    .any(|e| e.outcome == CallOutcome::Interrupted && e.ended_at_ms.is_none())
            );
            // Native observation IDs restart with the process; durable random IDs do not.
            reopened.observe(started(1)).unwrap();
            assert_eq!(reopened.entries(HistoryFilter::All).unwrap().len(), 3);
        }
        std::fs::write(&path, b"synthetic-corrupt-history").unwrap();
        assert!(HistoryDb::open(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic-corrupt-history");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
