//! Native hold, transfer and teardown regressions from the follow-up audit.
use super::*;
fn peer_dialog(lab: &Lab, invite: &str) -> String {
    format!(
        "From: {};tag=peer\r\nTo: {}\r\nCall-ID: {}\r\nContact: <{}>\r\n",
        header(invite, "To:"),
        header(invite, "From:"),
        header(invite, "Call-ID:"),
        lab.target
    )
}
fn select_original(lab: &mut Lab, original: &str) {
    lab.engine.switch_call(original).unwrap();
    let hold = lab.receive("child hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", &sdp(51, "recvonly"));
    let resume = lab.receive("original resume", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    lab.reply(&resume, "200 OK", "", &sdp(52, "sendrecv"));
    lab.wait(|s| s.call_id.as_deref() == Some(original) && s.calls.iter().all(|c| !c.hold_pending));
    lab.settle();
}
#[test]
fn unsupported_codec_in_hold_answer_must_not_confirm_success() {
    incompatible_answer_recovery(false);
}
#[test]
fn unsupported_codec_initial_answer_must_not_become_active() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-initial-codec");
    let invite = lab.outgoing();
    let bad = sdp(70, "sendrecv")
        .replace("RTP/AVP 0", "RTP/AVP 111")
        .replace("0 PCMU/8000", "111 opus/48000/2");
    lab.reply(&invite, "200 OK", "", &bad);
    lab.receive("ACK", |p| p.starts_with("ACK "));
    lab.settle();
    let state = lab.wait(|s| s.calls.is_empty());
    assert_eq!(state.call, CallState::Idle);
    assert_eq!(
        state.last_call_failure.unwrap().error.code,
        ErrorCode::NativeFailure
    );
}
#[test]
fn target_bye_during_switch_must_not_park_survivor() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-original-bye-switch");
    let dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (child, _) = lab.establish_consultation(&original);
    lab.engine.switch_call(&original).unwrap();
    let hold = lab.receive("hold child before original selection", |p| {
        p.starts_with("INVITE ")
    });
    lab.request(&dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.len() == 1);
    lab.reply(&hold, "200 OK", "", &sdp(60, "recvonly"));
    lab.receive("ACK", |p| p.starts_with("ACK "));
    lab.settle();
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(state.call_id.as_deref(), Some(child.as_str()));
    assert!(!state.calls[0].local_held);
    assert!(!state.calls[0].hold_pending);
}
#[test]
fn child_bye_during_original_hold_must_restore_original() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-child-bye-switch");
    lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (child, child_invite) = lab.establish_consultation(&original);
    select_original(&mut lab, &original);
    lab.engine.switch_call(&child).unwrap();
    let hold = lab.receive("hold original before child selection", |p| {
        p.starts_with("INVITE ")
    });
    let child_dialog = peer_dialog(&lab, &child_invite);
    lab.request(&child_dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.len() == 1);
    lab.reply(&hold, "200 OK", "", &sdp(61, "recvonly"));
    lab.receive("ACK", |p| p.starts_with("ACK "));
    lab.settle();
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(state.call_id.as_deref(), Some(original.as_str()));
    assert!(!state.calls[0].local_held);
    assert!(!state.calls[0].hold_pending);
}
#[test]
fn duplicate_final_transfer_notify_must_replay_200() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-notify-duplicate");
    let dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&original, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.notify_transfer_result(&dialog, &refer, 2, 486);
    lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
    lab.notify_transfer_result(&dialog, &refer, 2, 486);
}
#[test]
fn original_bye_before_success_notify_must_retain_transfer_outcome() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-bye-before-notify");
    let dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (child, _) = lab.establish_consultation(&original);
    lab.engine.complete_transfer(&child).unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.request(&dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.len() == 1);
    lab.notify_transfer_result(&dialog, &refer, 3, 200);
    lab.settle();
    let state = lab.wait(|s| s.calls.is_empty());
    assert_eq!(
        state
            .call_events
            .iter()
            .filter(|e| matches!(e.event, CallEventKind::TransferSucceeded))
            .count(),
        1
    );
}
#[test]
fn malformed_notify_body_must_not_end_call() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-malformed-notify");
    let dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&original, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: active;expires=60\r\nContent-Type: text/plain\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(
        &dialog,
        "NOTIFY",
        2,
        &event,
        "unrelated text SIP/2.0 200 OK\r\n",
    );
    lab.response(415, "2 NOTIFY");
    lab.settle();
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(
        state.calls.len(),
        1,
        "text/plain containing a status substring was mistaken for successful transfer"
    );
}
#[test]
fn malformed_offerless_reinvite_ack_must_not_be_silently_accepted() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-offerless-ack");
    let dialog = lab.incoming();
    lab.request(&dialog, "INVITE", 2, "", "");
    let response = lab.response(200, "2 INVITE");
    lab.request(
        &response,
        "ACK",
        2,
        "Content-Type: application/sdp\r\n",
        "this is not SDP\r\n",
    );
    lab.settle();
    let state = lab.wait(|s| s.calls.is_empty());
    assert_eq!(
        state.last_call_failure.unwrap().error.code,
        ErrorCode::NativeFailure
    );
}
fn hold_refusal_case(status: u16, was_held: bool) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new(&format!("scenario-hold-{status}-{was_held}"));
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    if was_held {
        lab.set_local_hold(&id, true);
    }
    lab.engine.set_call_held(&id, !was_held).unwrap();
    let offer = lab.receive("hold or resume", |p| p.starts_with("INVITE "));
    lab.reply(&offer, &format!("{status} Synthetic"), "", "");
    if status == 408 || status == 481 {
        lab.wait(|s| s.calls.is_empty());
    } else {
        let state = lab.wait(|s| s.calls[0].last_error.is_some() && !s.calls[0].hold_pending);
        assert_eq!(state.calls[0].local_held, was_held);
        assert_eq!(
            state.calls[0].last_error.as_ref().unwrap().sip_status,
            Some(status)
        );
        lab.engine.set_call_muted(&id, true).unwrap();
        lab.wait(|s| s.calls[0].muted);
    }
}
fn transfer_failure_case(status: u16, was_held: bool) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new(&format!("scenario-transfer-{status}-{was_held}"));
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    if was_held {
        lab.set_local_hold(&id, true);
    }
    lab.engine.transfer_call(&id, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.notify_transfer_result(&dialog, &refer, 2, status);
    let state = lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
    assert_eq!(state.calls[0].local_held, was_held);
    assert_eq!(
        state.calls[0].last_error.as_ref().unwrap().sip_status,
        Some(status)
    );
    lab.engine.set_call_muted(&id, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
}
#[test]
fn bye_during_hold_then_late_200_must_not_resurrect_call() {
    late_answer_after_end_case(true, false);
}
#[test]
fn pending_refer_allows_hangup_and_mute_but_blocks_sdp_controls() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-busy-controls");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&id, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Pending);
    assert_eq!(
        lab.engine.set_call_held(&id, true).unwrap_err().code,
        ErrorCode::Busy
    );
    assert_eq!(
        lab.engine.start_consultation(&id, "456").unwrap_err().code,
        ErrorCode::Busy
    );
    assert_eq!(
        lab.engine.transfer_call(&id, "456").unwrap_err().code,
        ErrorCode::Busy
    );
    lab.engine.set_call_muted(&id, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
    lab.engine.hangup_call(&id).unwrap();
    let bye = lab.receive("BYE during pending REFER", |p| p.starts_with("BYE "));
    lab.reply(&bye, "200 OK", "", "");
    lab.wait(|s| s.calls.is_empty());
}
#[test]
fn old_success_notify_must_not_complete_retry() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-late-retry-notify");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&id, "123").unwrap();
    let refer = lab.receive("first REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.notify_transfer_result(&dialog, &refer, 2, 486);
    lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
    lab.engine.transfer_call(&id, "456").unwrap();
    let retry = lab.receive("retry REFER", |p| p.starts_with("REFER "));
    lab.reply(&retry, "202 Accepted", "", "");
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: terminated;reason=noresource\r\nContent-Type: message/sipfrag\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(&dialog, "NOTIFY", 3, &event, "SIP/2.0 200 OK\r\n");
    lab.response(481, "3 NOTIFY");
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(state.calls.len(), 1);
    assert_eq!(state.calls[0].transfer, phone_core::TransferState::Pending);
    lab.notify_transfer_result(&dialog, &retry, 4, 486);
    lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
}
#[test]
fn offer_glare_during_local_hold_does_not_override_pending_request() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-real-glare");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let hold = lab.receive("local hold", |p| p.starts_with("INVITE "));
    lab.request(
        &dialog,
        "INVITE",
        2,
        "Content-Type: application/sdp\r\n",
        &sdp(65, "sendonly"),
    );
    lab.response(491, "2 INVITE");
    lab.reply(&hold, "200 OK", "", &sdp(66, "recvonly"));
    let state = lab.wait(|s| s.calls[0].local_held && !s.calls[0].hold_pending);
    assert!(!state.calls[0].remote_held);
}
#[test]
fn remote_update_glare_during_local_hold_does_not_override_pending_request() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-update-glare");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let hold = lab.receive("local hold", |p| p.starts_with("INVITE "));
    lab.request(
        &dialog,
        "UPDATE",
        2,
        "Content-Type: application/sdp\r\n",
        &sdp(65, "sendonly"),
    );
    lab.response(491, "2 UPDATE");
    lab.reply(&hold, "200 OK", "", &sdp(66, "recvonly"));
    let state = lab.wait(|s| s.calls[0].local_held && !s.calls[0].hold_pending);
    assert!(!state.calls[0].remote_held);
}
#[test]
fn duplicate_bye_replays_200_after_call_release() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-duplicate-bye");
    let dialog = lab.incoming();
    for _ in 0..2 {
        lab.request(&dialog, "BYE", 2, "", "");
        lab.response(200, "2 BYE");
    }
    lab.wait(|s| s.calls.is_empty());
}

#[test]
fn hold_response_403() {
    hold_refusal_case(403, false);
}

#[test]
fn resume_response_403() {
    hold_refusal_case(403, true);
}

#[test]
fn hold_response_408() {
    hold_refusal_case(408, false);
}

#[test]
fn resume_response_408() {
    hold_refusal_case(408, true);
}

#[test]
fn hold_response_481() {
    hold_refusal_case(481, false);
}

#[test]
fn resume_response_481() {
    hold_refusal_case(481, true);
}

#[test]
fn hold_response_488() {
    hold_refusal_case(488, false);
}

#[test]
fn resume_response_488() {
    hold_refusal_case(488, true);
}

#[test]
fn hold_response_491() {
    hold_refusal_case(491, false);
}

#[test]
fn resume_response_491() {
    hold_refusal_case(491, true);
}

#[test]
fn hold_response_500() {
    hold_refusal_case(500, false);
}

#[test]
fn resume_response_500() {
    hold_refusal_case(500, true);
}

#[test]
fn hold_response_503() {
    hold_refusal_case(503, false);
}

#[test]
fn resume_response_503() {
    hold_refusal_case(503, true);
}

#[test]
fn transfer_notify_403_active() {
    transfer_failure_case(403, false);
}

#[test]
fn transfer_notify_403_held() {
    transfer_failure_case(403, true);
}

#[test]
fn transfer_notify_408_active() {
    transfer_failure_case(408, false);
}

#[test]
fn transfer_notify_408_held() {
    transfer_failure_case(408, true);
}

#[test]
fn transfer_notify_481_active() {
    transfer_failure_case(481, false);
}

#[test]
fn transfer_notify_481_held() {
    transfer_failure_case(481, true);
}

#[test]
fn transfer_notify_486_active() {
    transfer_failure_case(486, false);
}

#[test]
fn transfer_notify_486_held() {
    transfer_failure_case(486, true);
}

#[test]
fn transfer_notify_488_active() {
    transfer_failure_case(488, false);
}

#[test]
fn transfer_notify_488_held() {
    transfer_failure_case(488, true);
}

#[test]
fn transfer_notify_500_active() {
    transfer_failure_case(500, false);
}

#[test]
fn transfer_notify_500_held() {
    transfer_failure_case(500, true);
}

#[test]
fn transfer_notify_503_active() {
    transfer_failure_case(503, false);
}

#[test]
fn transfer_notify_503_held() {
    transfer_failure_case(503, true);
}

#[test]
fn transfer_notify_603_active() {
    transfer_failure_case(603, false);
}

#[test]
fn transfer_notify_603_held() {
    transfer_failure_case(603, true);
}
#[test]
fn manual_hold_during_consultation_must_survive_child_bye() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-manual-hold-intent");
    lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (_child, invite) = lab.establish_consultation(&original);
    select_original(&mut lab, &original);
    lab.set_local_hold(&original, true);
    let child_dialog = peer_dialog(&lab, &invite);
    lab.request(&child_dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.len() == 1);
    lab.settle();
    let state = lab.engine.snapshot().unwrap();
    assert!(state.calls[0].local_held);
    assert!(!state.calls[0].hold_pending);
}
#[test]
fn explicit_switch_to_preheld_original_must_survive_child_bye() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-manual-resume-intent");
    lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.set_local_hold(&original, true);
    lab.engine.start_consultation(&original, "123").unwrap();
    let invite = lab.receive("child INVITE", |p| p.starts_with("INVITE "));
    lab.reply(&invite, "200 OK", "", &sdp(1, "sendrecv"));
    lab.wait(|s| {
        s.calls.len() == 2
            && s.call_id.as_deref() != Some(original.as_str())
            && s.call == CallState::Active
    });
    lab.settle();
    select_original(&mut lab, &original);
    let child_dialog = peer_dialog(&lab, &invite);
    lab.request(&child_dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.len() == 1);
    lab.settle();
    let state = lab.engine.snapshot().unwrap();
    assert!(
        !state.calls[0].local_held,
        "background consultation BYE re-held the actively selected original using obsolete intent"
    );
}
fn late_answer_after_end_case(remote: bool, provisional: bool) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new(&format!("scenario-late-answer-{remote}-{provisional}"));
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let hold = lab.receive("hold", |p| p.starts_with("INVITE "));
    if provisional {
        lab.reply(&hold, "100 Trying", "", "");
        thread::sleep(Duration::from_millis(30));
    }
    if remote {
        lab.request(&dialog, "BYE", 2, "", "");
        lab.response(200, "2 BYE");
    } else {
        lab.engine.hangup_call(&id).unwrap();
    }
    lab.wait(|s| s.calls.is_empty());
    lab.reply(&hold, "200 OK", "", &sdp(65, "recvonly"));
    let cseq = header(&hold, "CSeq:").replace("INVITE", "ACK");
    let ack = lab.receive("ACK of late 200 after local/remote end", |p| {
        p.starts_with("ACK ") && header(p, "CSeq:") == cseq
    });
    lab.reply(&hold, "200 OK", "", &sdp(65, "recvonly"));
    let replay = lab.receive("replayed ACK after end", |p| {
        p.starts_with("ACK ") && header(p, "CSeq:") == cseq
    });
    assert_eq!(ack, replay);
    lab.settle();
    assert!(lab.engine.snapshot().unwrap().calls.is_empty());
}
#[test]
fn late_answer_after_remote_bye_with_100() {
    late_answer_after_end_case(true, true);
}
#[test]
fn late_answer_after_local_hangup_without_100() {
    late_answer_after_end_case(false, false);
}
#[test]
fn late_answer_after_local_hangup_with_100() {
    late_answer_after_end_case(false, true);
}
#[test]
fn unsupported_codec_resume_answer_must_not_confirm_active() {
    incompatible_answer_recovery(true);
}
fn invalid_notify_case(content_type: &str, body: &str) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-isolated-notify");
    let dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&original, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: active;expires=60\r\nContent-Type: {content_type}\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(&dialog, "NOTIFY", 2, &event, body);
    lab.response(
        if content_type == "message/sipfrag" {
            400
        } else {
            415
        },
        "2 NOTIFY",
    );
    lab.settle();
    assert_eq!(
        lab.engine.snapshot().unwrap().calls.len(),
        1,
        "malformed status line or unsupported body type ended call"
    );
}
#[test]
fn notify_wrong_content_type_alone_must_not_end_call() {
    invalid_notify_case("text/plain", "SIP/2.0 200 OK\r\n");
}
#[test]
fn notify_status_substring_alone_must_not_end_call() {
    invalid_notify_case("message/sipfrag", "unrelated text SIP/2.0 200 OK\r\n");
}

fn incompatible_answer_recovery(was_held: bool) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new(if was_held {
        "scenario-resume-codec"
    } else {
        "scenario-hold-codec"
    });
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    if was_held {
        lab.set_local_hold(&id, true);
    }
    lab.engine.set_call_held(&id, !was_held).unwrap();
    let offer = lab.receive("hold/resume offer", |p| p.starts_with("INVITE "));
    let bad = sdp(70, if was_held { "sendrecv" } else { "recvonly" })
        .replace("RTP/AVP 0", "RTP/AVP 111")
        .replace("0 PCMU/8000", "111 opus/48000/2");
    lab.reply(&offer, "200 OK", "", &bad);
    lab.receive("ACK of unacceptable answer", |p| p.starts_with("ACK "));
    let state = lab.wait(|s| s.calls[0].hold_pending && s.calls[0].last_error.is_some());
    assert_eq!(state.calls[0].local_held, was_held);
    let compensation = lab.receive("compensating offer", |p| {
        p.starts_with("INVITE ") && header(p, "CSeq:") != header(&offer, "CSeq:")
    });
    assert!(compensation.contains(if was_held { "a=sendonly" } else { "a=sendrecv" }));
    lab.reply(
        &compensation,
        "200 OK",
        "",
        &sdp(71, if was_held { "recvonly" } else { "sendrecv" }),
    );
    lab.receive("compensation ACK", |p| p.starts_with("ACK "));
    let state = lab.wait(|s| !s.calls[0].hold_pending);
    assert_eq!(state.calls[0].local_held, was_held);
    lab.engine.set_call_muted(&id, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
}

fn pending_transfer_after_original_bye(lab: &mut Lab) -> (String, String, String) {
    let dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (child, _) = lab.establish_consultation(&original);
    lab.engine.complete_transfer(&child).unwrap();
    let refer = lab.receive("attended REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.request(&dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.len() == 1 && s.calls[0].transfer == phone_core::TransferState::Pending);
    (child, dialog, refer)
}
#[test]
fn retired_transfer_failure_releases_peer_for_retry() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-retired-failure");
    let (child, dialog, refer) = pending_transfer_after_original_bye(&mut lab);
    lab.notify_transfer_result(&dialog, &refer, 3, 486);
    let state = lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
    assert_eq!(
        state.calls[0].last_error.as_ref().unwrap().sip_status,
        Some(486)
    );
    lab.engine.transfer_call(&child, "456").unwrap();
    let retry = lab.receive("surviving peer REFER", |p| p.starts_with("REFER "));
    assert_ne!(header(&retry, "Call-ID:"), header(&refer, "Call-ID:"));
    lab.reply(&retry, "403 Forbidden", "", "");
    lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
}
#[test]
fn retired_transfer_deadline_keeps_mute_and_hangup_available() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-retired-timeout");
    let (child, _, _) = pending_transfer_after_original_bye(&mut lab);
    assert_eq!(
        lab.engine.set_call_held(&child, true).unwrap_err().code,
        ErrorCode::Busy
    );
    lab.engine.set_call_muted(&child, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
    let until = Instant::now() + Duration::from_secs(16);
    while Instant::now() < until {
        lab.settle();
    }
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(state.calls[0].transfer, phone_core::TransferState::Failed);
    assert_eq!(
        state.calls[0].last_error.as_ref().unwrap().code,
        ErrorCode::NativeFailure
    );
    lab.engine.hangup_call(&child).unwrap();
    let bye = lab.receive("surviving call BYE", |p| p.starts_with("BYE "));
    lab.reply(&bye, "200 OK", "", "");
    lab.wait(|s| s.calls.is_empty());
}
#[test]
fn retired_transfer_result_cannot_end_a_replacement_call() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-retired-replacement");
    let (child, dialog, refer) = pending_transfer_after_original_bye(&mut lab);
    lab.engine.hangup_call(&child).unwrap();
    let bye = lab.receive("child BYE", |p| p.starts_with("BYE "));
    lab.reply(&bye, "200 OK", "", "");
    lab.wait(|s| s.calls.is_empty());
    lab.incoming();
    let new_id = lab.engine.snapshot().unwrap().call_id.unwrap();
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: terminated;reason=noresource\r\nContent-Type: message/sipfrag\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(&dialog, "NOTIFY", 3, &event, "SIP/2.0 200 OK\r\n");
    lab.receive("old subscription response", |p| {
        p.starts_with("SIP/2.0 ") && header(p, "CSeq:") == "3 NOTIFY"
    });
    lab.settle();
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(state.call_id.as_deref(), Some(new_id.as_str()));
    assert_eq!(state.call, CallState::Active);
    assert_eq!(state.calls[0].transfer, phone_core::TransferState::Idle);
    assert!(
        !state
            .call_events
            .iter()
            .any(|e| matches!(e.event, CallEventKind::TransferSucceeded))
    );
}
#[test]
fn stopping_retired_transfer_does_not_wait_for_its_deadline() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-retired-stop");
    pending_transfer_after_original_bye(&mut lab);
    let start = Instant::now();
    lab.engine.shutdown().unwrap();
    assert!(start.elapsed() < Duration::from_secs(3));
}
fn blind_result_after_remote_bye(status: u16) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-retired-blind");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&id, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.request(&dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.is_empty());
    lab.notify_transfer_result(&dialog, &refer, 3, status);
    lab.settle();
    let state = lab.engine.snapshot().unwrap();
    assert!(state.calls.is_empty());
    assert_eq!(
        state
            .call_events
            .iter()
            .filter(|e| e.call_id == id
                && if status == 200 {
                    matches!(e.event, CallEventKind::TransferSucceeded)
                } else {
                    matches!(e.event, CallEventKind::TransferFailed)
                })
            .count(),
        1
    );
}
#[test]
fn blind_result_after_remote_bye_success_remains_observable() {
    blind_result_after_remote_bye(200);
}
#[test]
fn blind_result_after_remote_bye_failure_remains_observable() {
    blind_result_after_remote_bye(486);
}

#[test]
fn late_refer_refusal_after_original_bye_releases_owner() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-retired-refer-refusal");
    let dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (child, _) = lab.establish_consultation(&original);
    lab.engine.complete_transfer(&child).unwrap();
    let refer = lab.receive("REFER before response", |p| p.starts_with("REFER "));
    lab.request(&dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    lab.wait(|s| s.calls.len() == 1);
    lab.reply(&refer, "403 Forbidden", "", "");
    let state = lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
    assert_eq!(
        state.calls[0].last_error.as_ref().unwrap().sip_status,
        Some(403)
    );
    lab.engine.set_call_muted(&child, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
}
fn late_terminal_hold_response_after_hangup(status: u16, provisional: bool) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-ended-hold-terminal");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let hold = lab.receive("hold before hangup", |p| p.starts_with("INVITE "));
    if provisional {
        lab.reply(&hold, "100 Trying", "", "");
        thread::sleep(Duration::from_millis(30));
    }
    lab.engine.hangup_call(&id).unwrap();
    lab.wait(|s| s.calls.is_empty());
    lab.reply(&hold, &format!("{status} Synthetic"), "", "");
    let bye = lab.receive_for(
        Duration::from_secs(1),
        "retained session released to finish local BYE",
        |p| p.starts_with("BYE "),
    );
    lab.reply(&bye, "200 OK", "", "");
    assert!(lab.engine.snapshot().unwrap().calls.is_empty());
}
#[test]
fn late_hold_481_after_hangup_without_100_finishes_teardown() {
    late_terminal_hold_response_after_hangup(481, false);
}
#[test]
fn late_hold_481_after_hangup_with_100_finishes_teardown() {
    late_terminal_hold_response_after_hangup(481, true);
}
#[test]
fn late_hold_408_after_hangup_without_100_finishes_teardown() {
    late_terminal_hold_response_after_hangup(408, false);
}
#[test]
fn late_hold_408_after_hangup_with_100_finishes_teardown() {
    late_terminal_hold_response_after_hangup(408, true);
}
#[test]
fn notify_without_complete_status_line_cannot_complete_transfer() {
    invalid_notify_case("message/sipfrag", "SIP/2.0 200 OK");
}
#[test]
fn notify_with_lone_lf_cannot_complete_transfer() {
    invalid_notify_case("message/sipfrag", "SIP/2.0 200 OK\n");
}
#[test]
fn notify_with_empty_reason_phrase_can_complete_transfer() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("scenario-empty-reason");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&id, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: terminated;reason=noresource\r\nContent-Type: message/sipfrag\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(&dialog, "NOTIFY", 2, &event, "SIP/2.0 200 \r\n");
    lab.response(200, "2 NOTIFY");
    lab.settle();
    let state = lab.wait(|s| s.calls.is_empty());
    assert!(
        state
            .call_events
            .iter()
            .any(|e| matches!(e.event, CallEventKind::TransferSucceeded))
    );
}
