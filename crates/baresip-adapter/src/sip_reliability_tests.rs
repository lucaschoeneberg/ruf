//! Native regressions for the SIP reliability audit.
use super::*;

#[test]
fn early_media_tone_detection_uses_fresh_windows() {
    let tone: Vec<i16> = (0..16000)
        .map(|i| if i % 16 < 8 { 9852 } else { -9852 })
        .collect();
    let mut interrupted = tone.clone();
    interrupted[14880..].fill(0);
    assert!(contains_500_hz_pcm(&interrupted));
    assert!(!contains_500_hz_pcm(&vec![0; 16000]));
    let wrong_tone: Vec<i16> = (0..16000)
        .map(|i| if i % 20 < 10 { 9852 } else { -9852 })
        .collect();
    assert!(!contains_500_hz_pcm(&wrong_tone));
    let quiet_tone: Vec<i16> = (0..16000)
        .map(|i| if i % 16 < 8 { 20 } else { -20 })
        .collect();
    assert!(!contains_500_hz_pcm(&quiet_tone));
    let mut buffered = vec![0; 16000];
    buffered[..400].copy_from_slice(&tone[..400]);
    assert!(!contains_500_hz_pcm(&buffered));
    assert!(!contains_500_hz_pcm(&tone[..7999]));
}

#[test]
fn early_media_pcm_continues_after_sdp_less_ringing() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let recording = std::env::temp_dir().join("sip-events-early-pcm.wav");
    if let Err(error) = std::fs::remove_file(&recording) {
        assert_eq!(error.kind(), ErrorKind::NotFound, "{error}");
    }
    let mut lab = Lab::new("early-pcm");
    let invite = lab.outgoing();
    let port: u16 = invite
        .lines()
        .find_map(|line| line.strip_prefix("m=audio "))
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let rtp = UdpSocket::bind("127.0.0.1:0").unwrap();
    let body = sdp(1, "sendrecv").replace(
        "m=audio 9000",
        &format!("m=audio {}", rtp.local_addr().unwrap().port()),
    );
    lab.reply(&invite, "183 Session Progress", "", &body);
    lab.wait(|s| s.call == CallState::EarlyMedia);
    let mut sequence = 0_u16;
    // The pre-ringing 400Hz tone cannot satisfy the post-ringing 500Hz check.
    assert!(
        stream_tone_until(&rtp, port, &mut sequence, 10, || {
            std::fs::metadata(&recording).is_ok_and(|m| m.len() >= 32000)
        }),
        "pre-ringing PCM timed out"
    );
    let baseline = std::fs::metadata(&recording).unwrap().len() as usize;
    assert!(baseline > 16000);
    lab.reply(&invite, "180 Ringing", "", "");
    lab.settle();
    assert!(
        stream_tone_until(&rtp, port, &mut sequence, 8, || recorded_500_hz_pcm(
            &recording, baseline
        )),
        "fresh 500Hz PCM after 180 timed out: baseline={baseline}, recorded={:?}",
        std::fs::metadata(&recording).map(|m| m.len())
    );
    let bytes = std::fs::read(&recording).unwrap();
    assert!(
        bytes.len() >= baseline + 16000,
        "no fresh PCM after 183 -> 180: before={baseline}, after={}",
        bytes.len()
    );
    let samples: Vec<i16> = bytes[baseline..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect();
    assert!(
        contains_500_hz_pcm(&samples),
        "one second of fresh 500Hz PCM required after 180, captured {} samples",
        samples.len()
    );
    assert_eq!(lab.engine.snapshot().unwrap().call, CallState::EarlyMedia);
}

#[test]
fn early_fork_selection_preserves_target_refresh_and_later_fork_cleanup() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("early-fork");
    let invite = lab.outgoing();
    lab.reply(&invite, "180 Ringing", "", "");
    lab.wait(|s| s.call == CallState::Ringing);
    let endpoint = lab.peer.local_addr().unwrap();
    let response = |tag: &str| {
        format!(
            "SIP/2.0 200 OK\r\nVia: {}\r\nFrom: {}\r\nTo: {};tag={tag}\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <sip:{tag}@{endpoint}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{}",
            header(&invite, "Via:"),
            header(&invite, "From:"),
            header(&invite, "To:"),
            header(&invite, "Call-ID:"),
            header(&invite, "CSeq:"),
            sdp(1, "sendrecv").len(),
            sdp(1, "sendrecv")
        )
    };
    lab.send(&response("selected-fork"));
    lab.receive("selected fork ACK", |p| {
        p.starts_with("ACK ") && header(p, "To:").contains("selected-fork")
    });
    let id = lab.wait(|s| s.call == CallState::Active).call_id.unwrap();
    lab.send(&response("discarded-fork"));
    lab.receive("later fork ACK", |p| {
        p.starts_with("ACK ") && header(p, "To:").contains("discarded-fork")
    });
    let bye = lab.receive("later fork BYE", |p| {
        p.starts_with("BYE ") && header(p, "To:").contains("discarded-fork")
    });
    lab.reply(&bye, "200 OK", "", "");
    for (version, held, direction) in [(2, true, "recvonly"), (3, false, "sendrecv")] {
        lab.engine.set_call_held(&id, held).unwrap();
        let refresh = lab.receive("selected fork target refresh", |p| p.starts_with("INVITE "));
        assert_eq!(
            refresh
                .lines()
                .filter(|line| line.starts_with("To:"))
                .count(),
            1
        );
        assert_eq!(
            refresh
                .lines()
                .filter(|line| line.starts_with("From:"))
                .count(),
            1
        );
        assert!(header(&refresh, "To:").contains("selected-fork"));
        lab.reply(&refresh, "200 OK", "", &sdp(version, direction));
        lab.wait(|s| !s.calls[0].hold_pending && s.calls[0].local_held == held);
    }
}
impl Lab {
    fn offer_with_sdp(&mut self, body: &str, extra: &str) -> String {
        self.incoming_sequence += 1;
        let sequence = self.incoming_sequence;
        let endpoint = self.peer.local_addr().unwrap();
        let content_type = if body.is_empty() {
            ""
        } else {
            "Content-Type: application/sdp\r\n"
        };
        self.send(&format!(
            "INVITE {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKaudit-initial-{sequence};rport\r\nMax-Forwards: 70\r\nFrom: \"Audit Caller\" <sip:12345@lab>;tag=caller\r\nTo: <{}>\r\nCall-ID: synthetic-audit-incoming-{sequence}\r\nCSeq: 1 INVITE\r\nContact: <sip:caller@{endpoint}>\r\n{extra}{content_type}Content-Length: {}\r\n\r\n{body}", self.target, self.target, body.len()));
        self.receive("initial response", |p| {
            p.starts_with("SIP/2.0 ") && header(p, "CSeq:") == "1 INVITE"
        })
    }
    fn change_remote_sdp(&self, dialog: &str, sequence: u32, method: &str, body: &str) -> String {
        self.request(
            dialog,
            method,
            sequence,
            "Content-Type: application/sdp\r\n",
            body,
        );
        let response = self.receive("SDP update response", |p| {
            p.starts_with("SIP/2.0 ") && header(p, "CSeq:") == format!("{sequence} {method}")
        });
        if method == "INVITE" && response.starts_with("SIP/2.0 200 ") {
            self.request(&response, "ACK", sequence, "", "");
        }
        self.settle();
        response
    }
}

#[test]
fn early_media_followed_by_ringing_keeps_early_media() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-183-180");
    let invite = lab.outgoing();
    lab.reply(&invite, "183 Session Progress", "", &sdp(1, "sendrecv"));
    lab.wait(|s| s.call == CallState::EarlyMedia);
    lab.reply(&invite, "180 Ringing", "", "");
    lab.settle();
    assert_eq!(lab.engine.snapshot().unwrap().call, CallState::EarlyMedia);
}
#[test]
fn final_200_without_sdp_does_not_claim_active() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-no-sdp");
    let invite = lab.outgoing();
    lab.reply(&invite, "200 OK", "", "");
    lab.receive("ACK", |p| p.starts_with("ACK "));
    lab.settle();
    assert_ne!(
        lab.engine.snapshot().unwrap().call,
        CallState::Active,
        "no SDP answer was ever received"
    );
}
#[test]
fn incompatible_codec_update_preserves_existing_session() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-codec");
    let dialog = lab.incoming();
    let body = sdp(2, "sendonly")
        .replace("RTP/AVP 0", "RTP/AVP 111")
        .replace("0 PCMU/8000", "111 opus/48000/2");
    let response = lab.change_remote_sdp(&dialog, 2, "INVITE", &body);
    assert!(
        response.starts_with("SIP/2.0 488 "),
        "reject incompatible replacement of the only audio stream"
    );
    assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Active);
}

#[test]
fn duplicate_info_does_not_repeat_digit() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-info-duplicate");
    let dialog = lab.incoming();
    for _ in 0..2 {
        lab.request(
            &dialog,
            "INFO",
            2,
            "Content-Type: application/dtmf-relay\r\n",
            "Signal=5\r\nDuration=10\r\n",
        );
        lab.response(200, "2 INFO");
        thread::sleep(Duration::from_millis(25));
    }
    let n = lab
        .engine
        .snapshot()
        .unwrap()
        .call_events
        .iter()
        .filter(|e| matches!(e.event, CallEventKind::DtmfStart { .. }))
        .count();
    assert_eq!(n, 1, "one INFO retransmission must not duplicate a digit");
}
#[test]
fn service_code_hash_is_uri_escaped() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-hash");
    lab.engine.dial("*21*123#").unwrap();
    let invite = lab.receive("service-code INVITE", |p| p.starts_with("INVITE "));
    assert!(
        invite.lines().next().unwrap().contains("*21*123%23@"),
        "SIP URI must escape #; actual {}",
        invite.lines().next().unwrap()
    );
}

#[test]
fn forked_2xx_acknowledges_and_ends_extra_dialog() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-fork");
    let invite = lab.outgoing();
    lab.reply(&invite, "200 OK", "", &sdp(1, "sendrecv"));
    lab.receive("first ACK", |p| p.starts_with("ACK "));
    lab.wait(|s| s.call == CallState::Active);
    lab.settle();
    let endpoint = lab.peer.local_addr().unwrap();
    let fork_response = format!(
        "SIP/2.0 200 OK\r\nVia: {}\r\nFrom: {}\r\nTo: {};tag=second-fork\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <sip:second-fork@{endpoint}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{}",
        header(&invite, "Via:"),
        header(&invite, "From:"),
        header(&invite, "To:"),
        header(&invite, "Call-ID:"),
        header(&invite, "CSeq:"),
        sdp(1, "sendrecv").len(),
        sdp(1, "sendrecv")
    );
    lab.send(&fork_response);
    let ack = lab.receive("ACK for extra confirmed dialog", |p| {
        p.starts_with("ACK ") && header(p, "To:").contains("second-fork")
    });
    let bye = lab.receive("BYE for extra confirmed dialog", |p| {
        p.starts_with("BYE ") && header(p, "To:").contains("second-fork")
    });
    lab.reply(&bye, "200 OK", "", "");
    lab.send(&fork_response);
    let replay = lab.receive("cached fork ACK", |p| {
        p.starts_with("ACK ") && header(p, "To:").contains("second-fork")
    });
    assert_eq!(ack, replay);
    let until = Instant::now() + Duration::from_millis(200);
    while Instant::now() < until {
        if let Some((packet, _)) = Lab::read(&lab.peer) {
            assert!(
                !packet.starts_with("BYE ") || !header(&packet, "To:").contains("second-fork"),
                "duplicate 2xx generated another BYE"
            );
            lab.service(&packet);
        }
    }
    assert_eq!(lab.engine.snapshot().unwrap().calls.len(), 1);
    assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Active);
}

#[test]
fn invalid_multi_digit_info_is_rejected() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-info-multi");
    let dialog = lab.incoming();
    lab.request(
        &dialog,
        "INFO",
        2,
        "Content-Type: application/dtmf-relay\r\n",
        "Signal=123\r\nDuration=100\r\n",
    );
    lab.response(400, "2 INFO");
}

#[test]
fn malformed_info_fields_are_not_silently_truncated() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("info-fields");
    let dialog = lab.incoming();
    for (sequence, body) in [
        "Signal=1x\r\nDuration=100\r\n",
        "Signal=5\r\nDuration=100x\r\n",
        "Signal=5\r\nDuration=0\r\n",
        "Signal=5\r\nDuration=5001\r\n",
        "Signal=5\r\nDuration=4294967296\r\n",
    ]
    .into_iter()
    .enumerate()
    {
        let sequence = sequence as u32 + 2;
        lab.request(
            &dialog,
            "INFO",
            sequence,
            "Content-Type: application/dtmf-relay\r\n",
            body,
        );
        lab.response(400, &format!("{sequence} INFO"));
    }
    assert!(lab.engine.snapshot().unwrap().call_events.is_empty());
}

impl Lab {
    fn set_local_hold(&mut self, id: &str, held: bool) {
        self.engine.set_call_held(id, held).unwrap();
        let offer = self.receive("local hold/resume offer", |p| p.starts_with("INVITE "));
        self.reply(
            &offer,
            "200 OK",
            "",
            &sdp(40, if held { "recvonly" } else { "sendrecv" }),
        );
        self.receive("hold/resume ACK", |p| {
            p.starts_with("ACK ") && header(p, "Call-ID:") == header(&offer, "Call-ID:")
        });
        self.wait(|s| {
            s.calls
                .iter()
                .any(|c| c.id == id && !c.hold_pending && c.local_held == held)
        });
        self.settle();
    }
    fn establish_consultation(&mut self, original: &str) -> (String, String) {
        self.engine.start_consultation(original, "123").unwrap();
        let hold = self.receive("hold before consult", |p| p.starts_with("INVITE "));
        self.reply(&hold, "200 OK", "", &sdp(40, "recvonly"));
        let invite = self.receive("child INVITE", |p| {
            p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
        });
        self.reply(
            &invite,
            "200 OK",
            "Supported: replaces\r\n",
            &sdp(1, "sendrecv"),
        );
        self.receive("child ACK", |p| {
            p.starts_with("ACK ") && header(p, "Call-ID:") == header(&invite, "Call-ID:")
        });
        let state = self.wait(|s| {
            s.calls.len() == 2
                && s.call == CallState::Active
                && s.call_id.as_deref() != Some(original)
        });
        self.settle();
        (state.call_id.unwrap(), invite)
    }
    fn notify_transfer_result(&self, dialog: &str, refer: &str, sequence: u32, status: u16) {
        let event = format!(
            "Event: refer;id={}\r\nSubscription-State: terminated;reason=noresource\r\nContent-Type: message/sipfrag\r\n",
            header(refer, "CSeq:").split_whitespace().next().unwrap()
        );
        self.request(
            dialog,
            "NOTIFY",
            sequence,
            &event,
            &format!("SIP/2.0 {status} Synthetic\r\n"),
        );
        self.response(200, &format!("{sequence} NOTIFY"));
    }
}
#[test]
fn blind_refer_refusal_preserves_explicit_hold_intent() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for was_held in [false, true] {
        let mut lab = Lab::new("audit-refusal-hold");
        let dialog = lab.incoming();
        let id = lab.engine.snapshot().unwrap().call_id.unwrap();
        if was_held {
            lab.set_local_hold(&id, true);
        }
        lab.engine.transfer_call(&id, "123").unwrap();
        let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
        lab.reply(&refer, "403 Forbidden", "", "");
        let state = lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
        assert_eq!(state.calls[0].local_held, was_held);
        assert_eq!(
            state.calls[0].last_error.as_ref().unwrap().sip_status,
            Some(403)
        );
        assert_eq!(state.call_id.as_deref(), Some(id.as_str()));
        lab.end_incoming(&dialog, 2);
    }
}
#[test]
fn blind_notify_failure_preserves_explicit_hold_intent() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for was_held in [false, true] {
        let mut lab = Lab::new("audit-notify-hold");
        let dialog = lab.incoming();
        let id = lab.engine.snapshot().unwrap().call_id.unwrap();
        if was_held {
            lab.set_local_hold(&id, true);
        }
        lab.engine.transfer_call(&id, "123").unwrap();
        let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
        lab.reply(&refer, "202 Accepted", "", "");
        lab.notify_transfer_result(&dialog, &refer, 2, 486);
        let state = lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
        assert_eq!(state.calls[0].local_held, was_held);
        assert_eq!(
            state.calls[0].last_error.as_ref().unwrap().sip_status,
            Some(486)
        );
        lab.end_incoming(&dialog, 3);
    }
}
#[test]
fn attended_failure_keeps_both_dialogs_and_retry_works() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-attended-refusal");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (child, _) = lab.establish_consultation(&id);
    lab.engine.complete_transfer(&child).unwrap();
    let refer = lab.receive("attended REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    lab.notify_transfer_result(&dialog, &refer, 2, 486);
    let state = lab.wait(|s| {
        s.calls
            .iter()
            .any(|c| c.transfer == phone_core::TransferState::Failed)
    });
    assert_eq!(state.calls.len(), 2);
    assert!(state.calls[0].local_held);
    assert!(!state.calls[1].local_held);
    assert_eq!(state.call_id.as_deref(), Some(child.as_str()));
    lab.engine.complete_transfer(&child).unwrap();
    let retry = lab.receive("retry REFER", |p| p.starts_with("REFER "));
    lab.reply(&retry, "202 Accepted", "", "");
    lab.notify_transfer_result(&dialog, &retry, 3, 200);
    lab.settle();
    lab.wait(|s| s.calls.is_empty());
}

#[test]
fn hold_481_does_not_keep_nonexistent_dialog() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-481");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let hold = lab.receive("hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "481 Call/Transaction Does Not Exist", "", "");
    lab.settle();
    // Native teardown joins audio workers; settling the peer is not a barrier.
    lab.wait(|s| s.calls.is_empty());
}
#[test]
fn missing_sdp_accepted_hold_requires_reconciliation() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-invalid-accepted-hold");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let hold = lab.receive("hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", "");
    lab.receive("ACK", |p| p.starts_with("ACK "));
    let recovery = lab.receive("confirmed recovery after invalid accepted hold", |p| {
        p.starts_with("INVITE ") && header(p, "CSeq:") != header(&hold, "CSeq:")
    });
    assert!(lab.engine.snapshot().unwrap().calls[0].hold_pending);
    lab.reply(&recovery, "200 OK", "", &sdp(42, "sendrecv"));
    let state = lab.wait(|s| !s.calls[0].hold_pending);
    assert!(!state.calls[0].local_held);
}

#[test]
fn long_local_hold_resume_gets_full_media_grace_interval() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-long-local-hold");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.set_local_hold(&id, true);
    let until = Instant::now() + Duration::from_secs(65);
    while Instant::now() < until {
        lab.settle();
        assert_eq!(
            lab.engine.snapshot().unwrap().calls.len(),
            1,
            "hold may legitimately be silent"
        );
    }
    lab.engine.set_call_held(&id, false).unwrap();
    let resume = lab.receive("resume after long local hold", |p| p.starts_with("INVITE "));
    lab.reply(&resume, "200 OK", "", &sdp(41, "sendrecv"));
    lab.receive("resume ACK", |p| p.starts_with("ACK "));
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        lab.settle();
        let state = lab.engine.snapshot().unwrap();
        assert_eq!(
            state.calls.len(),
            1,
            "local hold must not consume the resume receive grace interval; terminal={:?}",
            state.last_call_failure
        );
    }
}

#[test]
fn rejected_offer_does_not_break_next_remote_hold_detection() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-partial-sdp");
    let dialog = lab.incoming();
    let invalid = sdp(2, "sendonly") + "a=rtpmap:malformed\r\n";
    lab.request(
        &dialog,
        "INVITE",
        2,
        "Content-Type: application/sdp\r\n",
        &invalid,
    );
    lab.response(488, "2 INVITE");
    assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Active);
    let valid = lab.change_remote_sdp(&dialog, 3, "INVITE", &sdp(3, "sendonly"));
    assert!(valid.starts_with("SIP/2.0 200 "));
    assert_eq!(
        lab.engine.snapshot().unwrap().call,
        CallState::Held,
        "the accepted hold following rejected SDP must be observed"
    );
}
#[test]
fn rejected_offer_does_not_break_next_remote_resume_detection() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-partial-resume");
    let dialog = lab.incoming();
    lab.change_remote_sdp(&dialog, 2, "INVITE", &sdp(2, "sendonly"));
    assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Held);
    let invalid = sdp(3, "sendrecv") + "a=rtpmap:malformed\r\n";
    lab.request(
        &dialog,
        "INVITE",
        3,
        "Content-Type: application/sdp\r\n",
        &invalid,
    );
    lab.response(488, "3 INVITE");
    let valid = lab.change_remote_sdp(&dialog, 4, "INVITE", &sdp(4, "sendrecv"));
    assert!(valid.starts_with("SIP/2.0 200 "));
    assert_eq!(
        lab.engine.snapshot().unwrap().call,
        CallState::Active,
        "the accepted resume following rejected SDP must be observed"
    );
}
#[test]
fn third_incoming_offer_is_recorded_during_consultation() {
    use std::sync::{Arc, Mutex};
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-third-history");
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    lab.engine
        .set_call_observer(Some(Arc::new(move |e| sink.lock().unwrap().push(e))));
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    let (_child, _) = lab.establish_consultation(&id);
    let before = events.lock().unwrap().len();
    let response = lab.offer_with_sdp(&sdp(1, "sendrecv"), "");
    assert!(response.starts_with("SIP/2.0 486 "));
    lab.settle();
    assert_eq!(lab.engine.snapshot().unwrap().calls.len(), 2);
    let actual = events.lock().unwrap().len();
    assert_eq!(
        actual,
        before + 2,
        "third busy offer must produce independent started/finished history facts"
    );
}
#[test]
fn transfer_hash_is_escaped_in_refer_to() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-transfer-hash");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&id, "*21*123#").unwrap();
    let refer = lab.receive("service-code REFER", |p| p.starts_with("REFER "));
    assert!(
        header(&refer, "Refer-To:").contains("%23"),
        "Refer-To: {}",
        header(&refer, "Refer-To:")
    );
}

#[test]
fn explicit_consult_cancel_resumes_even_preheld_original() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("audit-cancel-preheld");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.set_local_hold(&id, true);
    lab.engine.start_consultation(&id, "123").unwrap();
    let invite = lab.receive("consultation without redundant hold", |p| {
        p.starts_with("INVITE ")
    });
    assert_ne!(header(&invite, "Call-ID:"), header(&dialog, "Call-ID:"));
    lab.reply(&invite, "200 OK", "", &sdp(1, "sendrecv"));
    let state = lab.wait(|s| {
        s.calls.len() == 2
            && s.call_id.as_deref() != Some(id.as_str())
            && s.call == CallState::Active
    });
    let child = state.call_id.unwrap();
    lab.settle();
    lab.engine.cancel_consultation(&child).unwrap();
    let resume = lab.receive("explicit return resumes preheld original", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") == header(&dialog, "Call-ID:")
    });
    lab.reply(&resume, "200 OK", "", &sdp(42, "sendrecv"));
    let state = lab.wait(|s| s.calls.len() == 1 && !s.calls[0].hold_pending);
    assert!(!state.calls[0].local_held);
}

#[path = "sip_scenario_tests.rs"]
mod scenarios;
