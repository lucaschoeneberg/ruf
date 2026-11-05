/* Exercise the production queue handler without opening an OS audio device.
 * Only the external ua_answer result is injected; ringtone ownership uses libre. */
#define ua_answer test_ua_answer
#include "../bridge.c"
#include <assert.h>

static int answer_result;
static unsigned ring_destructions, answer_failures, errors, failed_dials;
static uint64_t failed_dial_generation;

int test_ua_answer(struct ua *ua, struct call *call, enum vidmode mode) {
    assert(ua && call && mode == VIDMODE_OFF);
    return answer_result;
}
int sip_audio_device_available(const char *id, int input) {
    (void)id; (void)input;
    return 0;
}
void sip_network_reset(void) {}
int sip_network_poll(bool active) { (void)active; return 0; }

static void ring_destroyed(void *arg) { (void)arg; ++ring_destructions; }
static void observed(const struct sip_event *event, void *arg) {
    (void)arg;
    if (event->kind == EV_ANSWER_FAILED) ++answer_failures;
    if (event->kind == EV_ERROR) ++errors;
    if (event->kind == EV_DIAL_FAILED) {
        ++failed_dials;
        failed_dial_generation = event->call_generation;
    }
}
static void execute(struct engine *engine, int operation) {
    struct request *request = calloc(1, sizeof(*request));
    assert(request);
    request->operation = operation;
    request->call_generation = engine->call_generation;
    queue_handler(operation, request, engine);
}

int main(void) {
    struct engine engine = {0};
    LOCK_INIT(&engine.lock);
    engine.audio_mode = 1;
    engine.callback = observed;
    engine.call_generation = 42;
    /* Opaque sentinels are only inspected by the injected answer function. */
    engine.call = (struct call *)&engine;
    engine.call_ua = (struct ua *)&engine;
    engine.ring = mem_zalloc(1, ring_destroyed);
    assert(engine.ring);
    answer_result = EAGAIN;
    execute(&engine, OP_ANSWER);
    assert(answer_failures == 1);
    assert(engine.ring && ring_destructions == 0);
    assert(!engine.call_answered);

    answer_result = 0;
    execute(&engine, OP_ANSWER);
    assert(!engine.ring && ring_destructions == 1);
    assert(engine.call_answered);

    /* Another already-queued hangup of the same closed generation is harmless. */
    engine.call = NULL;
    engine.call_ua = NULL;
    execute(&engine, OP_HANGUP);
    assert(errors == 0);
    /* An outgoing attempt losing the slot race must only fail its own token. */
    engine.call = (struct call *)&engine;
    engine.call_ua = (struct ua *)&engine;
    struct request *dial = calloc(1, sizeof(*dial));
    assert(dial);
    dial->operation = OP_DIAL;
    dial->call_generation = 43;
    queue_handler(OP_DIAL, dial, &engine);
    assert(failed_dials == 1 && failed_dial_generation == 43);
    assert(engine.call && engine.call_generation == 42);
    assert(errors == 0);
    LOCK_CLOSE(&engine.lock);
    puts("call_actions: answer failure retains ringtone; queued duplicate hangup and losing dial preserve the current call");
    return 0;
}
