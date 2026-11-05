/* Event-thread-only two-dialog controller. Included after engine/emit helpers. */
static void control_result(struct call *call, int kind, int status, void *arg);
enum { RECOVERY_NONE, RECOVERY_WAITING, RECOVERY_QUEUED, RECOVERY_RUNNING };
enum { PENDING_HOLD_QUEUED = 3 };
static void fail_recovery(struct engine *e, struct native_dialog *dialog, int status) {
    e->current = dialog;
    dialog->pending = dialog->recovery = 0;
    e->control_action = 0;
    e->action_target = NULL;
    emit(e, EV_RECOVERY_FAILED, status);
    ua_hangup(dialog->ua, dialog->call, 0, NULL);
}

static void select_dialog(struct engine *e, struct native_dialog *dialog) {
    e->current = e->selected = dialog;
    emit(e, EV_SELECTED, 0);
}
static bool controls_busy(const struct engine *e) {
    return e->control_action || e->dialogs[0].pending || e->dialogs[1].pending ||
        e->dialogs[0].recovery || e->dialogs[1].recovery;
}
/* Tones stay on the existing baresip media path. One bounded FIFO belongs to
 * the audible dialog; every stop/switch/close releases the key and cancels time. */
static struct native_dialog *dtmf_dialog(struct engine *e) {
    for (size_t i = 0; i < 2; ++i)
        if (e->dialogs[i].call && e->dialogs[i].generation == e->dtmf_generation) return &e->dialogs[i];
    return NULL;
}
static void stop_dtmf(struct engine *e) {
    tmr_cancel(&e->dtmf_timer);
    struct native_dialog *dialog = dtmf_dialog(e);
    if (e->dtmf_active && dialog) (void)call_send_digit(dialog->call, KEYCODE_REL);
    e->dtmf_active = false;
    e->dtmf_count = 0;
    e->dtmf_generation = 0;
    memset(e->dtmf_keys, 0, sizeof(e->dtmf_keys));
}
static void dtmf_tick(void *arg) {
    struct engine *e = arg;
    struct native_dialog *dialog = dtmf_dialog(e);
    if (!dialog || dialog != e->selected || call_state(dialog->call) != CALL_STATE_ESTABLISHED || call_is_onhold(dialog->call) || controls_busy(e)) { stop_dtmf(e); return; }
    if (e->dtmf_active) {
        (void)call_send_digit(dialog->call, KEYCODE_REL);
        e->dtmf_active = false;
        tmr_start(&e->dtmf_timer, 80, dtmf_tick, e);
    } else if (e->dtmf_count) {
        char key = e->dtmf_keys[0];
        memmove(e->dtmf_keys, e->dtmf_keys + 1, --e->dtmf_count);
        e->dtmf_keys[e->dtmf_count] = 0;
        int error = call_send_digit(dialog->call, key);
        if (error) {
            struct native_dialog *previous = e->current;
            e->current = dialog;
            emit(e, EV_CONTROL_FAILED, 0);
            e->current = previous;
            stop_dtmf(e);
        } else {
            e->dtmf_active = true;
            tmr_start(&e->dtmf_timer, 160, dtmf_tick, e);
        }
    } else stop_dtmf(e);
}
static void begin_hold(struct engine *e, struct native_dialog *dialog, bool held) {
    e->current = dialog;
    if (call_is_onhold(dialog->call) == held && !dialog->recovery) { emit(e, EV_LOCAL_HOLD_DONE, held); return; }
    if (!call_sdp_change_allowed(dialog->call)) {
        dialog->hold_target = held;
        dialog->pending = PENDING_HOLD_QUEUED;
        dialog->deadline = tmr_jiffies() + 10000;
        emit(e, EV_LOCAL_HOLD_PENDING, 0);
        return;
    }
    dialog->hold_target = held;
    dialog->pending = 1;
    dialog->deadline = tmr_jiffies() + 10000;
    emit(e, EV_LOCAL_HOLD_PENDING, 0);
    bool recovering = dialog->recovery != RECOVERY_NONE;
    if (recovering) dialog->recovery = RECOVERY_RUNNING;
    int error = recovering && call_is_onhold(dialog->call) == held ?
        call_refresh_hold(dialog->call) : call_hold(dialog->call, held);
    if (error) {
        call_cancel_control(dialog->call);
        if (dialog->call && dialog->pending && !dialog->recovery) {
            dialog->pending = 0;
            e->control_action = 0;
            e->action_target = NULL;
            emit(e, EV_CONTROL_FAILED, 0);
        }
    }
}
static void dialog_closed(struct engine *e, int status) {
    struct native_dialog *closed = e->current;
    if (e->dtmf_generation == closed->generation) stop_dtmf(e);
    struct native_dialog *other = closed == &e->dialogs[0] ? &e->dialogs[1] : &e->dialogs[0];
    bool consultation = closed->consultation;
    bool consultation_failed = consultation && !closed->answered && !closed->local_ending &&
        !e->stopping && (status == 0 || (status >= 300 && status != 487));
    memset(closed, 0, sizeof(*closed));
    e->control_action = 0;
    e->action_target = NULL;
    if (other->call) {
        other->consultation = false;
        select_dialog(e, other);
        if (consultation && !other->pending && !e->stopping) begin_hold(e, other, other->return_held);
        if (consultation_failed) emit(e, EV_CONSULT_FAILED, status <= 699 ? status : 0);
    }
}
static void control_result(struct call *call, int kind, int status, void *arg) {
    struct engine *e = arg;
    struct native_dialog *previous = e->current;
    struct native_dialog *dialog = NULL;
    for (size_t i = 0; i < 2; ++i) if (e->dialogs[i].call == call) dialog = &e->dialogs[i];
    if (!dialog || !dialog->pending) return;
    e->current = dialog;
    if (kind == 4) {
        e->control_action = 0;
        e->action_target = NULL;
        dialog->recovery = status ? RECOVERY_QUEUED : RECOVERY_WAITING;
        dialog->pending = status ? 0 : 1;
        dialog->deadline = tmr_jiffies() + (status ? 10000 : 12000);
        if (!status) emit(e, EV_RECOVERING, 0);
        e->current = previous;
        return;
    }
    dialog->pending = 0;
    if (kind == 1) {
        if (status >= 200 && status < 300) {
            dialog->recovery = RECOVERY_NONE;
            (void)stream_enable_rx(audio_strm(call_audio(call)), !dialog->hold_target);
            emit(e, EV_LOCAL_HOLD_DONE, dialog->hold_target);
        }
        else if (dialog->recovery == RECOVERY_RUNNING) { fail_recovery(e, dialog, status); }
        else { dialog->recovery = RECOVERY_NONE; e->control_action = 0; e->action_target = NULL; emit(e, EV_CONTROL_FAILED, status); }
    } else {
        emit(e, EV_TRANSFER_RESULT, status);
        if (kind == 2) {
            /* Only final 2xx NOTIFY can terminate the transferor's dialogs. */
            for (size_t i = 0; i < 2; ++i) if (e->dialogs[i].call) {
                e->current = &e->dialogs[i];
                /* Forward the confirmed session outcome before each dialog's
                 * cleanup so the final idle projection retains success. */
                if (e->current != dialog) emit(e, EV_TRANSFER_RESULT, status);
                emit(e, EV_LOCAL_END, 0);
                ua_hangup(e->current->ua, e->current->call, 0, NULL);
            }
        } else { emit(e, EV_TRANSFER_FAILED, status); }
    }
    e->current = previous;
}
static void control_tick(void *arg) {
    struct engine *e = arg;
    e->current = e->selected;
    for (size_t i = 0; i < 2; ++i) {
        struct native_dialog *dialog = &e->dialogs[i];
        if (dialog->call && dialog->pending == PENDING_HOLD_QUEUED) {
            if (tmr_jiffies() >= dialog->deadline) {
                e->current = dialog;
                dialog->pending = 0;
                e->control_action = 0;
                e->action_target = NULL;
                emit(e, EV_CONTROL_FAILED, 0);
            } else if (call_sdp_change_allowed(dialog->call)) {
                dialog->pending = 0;
                begin_hold(e, dialog, dialog->hold_target);
            }
            continue;
        }
        if (dialog->call && dialog->pending && tmr_jiffies() >= dialog->deadline) {
            e->current = dialog;
            int pending = dialog->pending;
            if (dialog->recovery) { fail_recovery(e, dialog, 0); continue; }
            call_cancel_control(dialog->call);
            if (!dialog->recovery) dialog->pending = 0;
            e->control_action = 0;
            if (pending == 2) { emit(e, EV_TRANSFER_RESULT, 0); emit(e, EV_TRANSFER_FAILED, 0); }
        }
        if (dialog->call && dialog->recovery == RECOVERY_QUEUED) {
            if (tmr_jiffies() >= dialog->deadline) { fail_recovery(e, dialog, 0); }
            else if (call_sdp_change_allowed(dialog->call)) {
                begin_hold(e, dialog, call_is_onhold(dialog->call));
            }
        }
    }
    if (e->control_action && !e->dialogs[0].pending && !e->dialogs[1].pending) {
        int action = e->control_action;
        struct native_dialog *target = e->action_target;
        e->control_action = 0;
        e->action_target = NULL;
        if (action == OP_CONSULT) {
            struct native_dialog *original = e->selected;
            struct native_dialog *next = original == &e->dialogs[0] ? &e->dialogs[1] : &e->dialogs[0];
            struct sip_account *account = find_account(e, original->account_id);
            if (account && !next->call && original->call && call_is_onhold(original->call)) {
                memset(next, 0, sizeof(*next));
                next->generation = sip_engine_reserve_call_generation(e);
                next->account_id = original->account_id;
                next->consultation = true;
                e->current = next;
                const struct sip_event started = { EV_DIAL_STARTED, 0, next->account_id, next->generation, NULL, e->consultation_destination, NULL };
                e->callback(&started, e->callback_arg);
                char uri[512];
                snprintf(uri, sizeof(uri), "sip:%s@%s;transport=%s", e->consultation_destination, account->registrar, transport_name(account->transport));
                int error = ua_connect(account->ua, &next->call, NULL, uri, VIDMODE_OFF);
                if (error) {
                    const struct sip_event failed = { EV_DIAL_FAILED, error, next->account_id, next->generation, NULL, NULL, NULL };
                    e->callback(&failed, e->callback_arg);
                    memset(next, 0, sizeof(*next));
                    e->current = original;
                    begin_hold(e, original, original->return_held);
                    emit(e, EV_CONSULT_FAILED, 0);
                } else { select_dialog(e, next); }
            }
        } else if (action == OP_SWITCH && target && target->call) {
            select_dialog(e, target);
            begin_hold(e, target, false);
        }
    }
    e->current = e->selected;
    tmr_start(&e->control_timer, 20, control_tick, e);
}
static void control_request(struct engine *e, struct request *r) {
    struct native_dialog *dialog = NULL;
    for (size_t i = 0; i < 2; ++i)
        if (e->dialogs[i].call && e->dialogs[i].generation == r->call_generation) dialog = &e->dialogs[i];
    if (!dialog) { emit(e, EV_STALE, 0); return; }
    e->current = dialog;
    if (r->operation != OP_MUTE && r->operation != OP_DTMF) stop_dtmf(e);
    if ((r->operation == OP_CANCEL_CONSULT || r->operation == OP_COMPLETE_TRANSFER) && !dialog->consultation) {
        emit(e, EV_STALE, 0);
        return;
    }
    if (r->operation != OP_MUTE && controls_busy(e)) { emit(e, EV_CONTROL_REJECTED, 0); return; }
    struct native_dialog *other = dialog == &e->dialogs[0] ? &e->dialogs[1] : &e->dialogs[0];
    int error = 0;
    if (r->operation != OP_CANCEL_CONSULT && call_state(dialog->call) != CALL_STATE_ESTABLISHED) error = EINVAL;
    else switch (r->operation) {
    case OP_DTMF:
        if (dialog != e->selected || call_is_onhold(dialog->call) || strlen(r->destination) != 1 || !strchr("0123456789*#", r->destination[0])) { error = EINVAL; break; }
        if (e->dtmf_generation && e->dtmf_generation != dialog->generation) stop_dtmf(e);
        if (e->dtmf_count >= sizeof(e->dtmf_keys)) { error = EBUSY; break; }
        e->dtmf_generation = dialog->generation;
        e->dtmf_keys[e->dtmf_count++] = r->destination[0];
        if (!e->dtmf_active && !tmr_isrunning(&e->dtmf_timer)) dtmf_tick(e);
        break;
    case OP_MUTE:
        dialog->muted = !strcmp(r->destination, "1");
        audio_mute(call_audio(dialog->call), dialog->muted);
        emit(e, EV_MUTED, dialog->muted);
        break;
    case OP_HOLD:
        if (!strcmp(r->destination, "0") && other->call && !call_is_onhold(other->call)) {
            e->control_action = OP_SWITCH;
            e->action_target = dialog;
            begin_hold(e, other, true);
        } else { begin_hold(e, dialog, !strcmp(r->destination, "1")); }
        break;
    case OP_CONSULT:
        if (other->call || strlen(r->destination) > 64) { error = EBUSY; break; }
        dialog->return_held = call_is_onhold(dialog->call);
        str_ncpy(e->consultation_destination, r->destination, sizeof(e->consultation_destination));
        e->control_action = OP_CONSULT;
        begin_hold(e, dialog, true);
        break;
    case OP_SWITCH:
        if (dialog == e->selected) break;
        if (!other->call) { error = EINVAL; break; }
        e->control_action = OP_SWITCH;
        e->action_target = dialog;
        begin_hold(e, other, true);
        break;
    case OP_CANCEL_CONSULT: {
        struct native_dialog *consultation = dialog->consultation ? dialog : other;
        if (!consultation->call || !consultation->consultation) { error = EINVAL; break; }
        struct native_dialog *parent = consultation == &e->dialogs[0] ? &e->dialogs[1] : &e->dialogs[0];
        parent->return_held = false;
        e->current = consultation;
        emit(e, EV_LOCAL_END, 0);
        ua_hangup(consultation->ua, consultation->call, 0, NULL);
        break;
    }
    case OP_TRANSFER:
    case OP_COMPLETE_TRANSFER:
        if (r->operation == OP_TRANSFER && other->call) { error = EBUSY; break; }
        if (r->operation == OP_COMPLETE_TRANSFER && (!other->call || call_state(other->call) != CALL_STATE_ESTABLISHED)) { error = EINVAL; break; }
        /* Attended transfer always sends REFER from original to consultation. */
        if (r->operation == OP_COMPLETE_TRANSFER && dialog->consultation) { struct native_dialog *swap = dialog; dialog = other; other = swap; e->current = dialog; }
        dialog->pending = 2;
        dialog->deadline = tmr_jiffies() + 15000;
        emit(e, EV_TRANSFER_PENDING, 0);
        error = r->operation == OP_TRANSFER ? call_transfer(dialog->call, r->destination) : call_replace_transfer(dialog->call, other->call);
        if (error) { call_cancel_control(dialog->call); dialog->pending = 0; emit(e, EV_TRANSFER_RESULT, 0); }
        break;
    default: error = EINVAL;
    }
    if (error) { e->control_action = 0; e->action_target = NULL; emit(e, EV_CONTROL_FAILED, 0); }
}
