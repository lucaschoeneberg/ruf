/* Private ABI owned by this project, compiled against the pinned upstream headers. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <inttypes.h>
#include "audio_devices.h"
#include "audio_diagnostics.h"
#include "network_roam.h"
#include "security.h"
#include <re.h>
#include <rem.h>
#include <baresip.h>
#include <re_atomic.h>

#ifdef WIN32
#include <windows.h>
typedef CRITICAL_SECTION queue_lock;
#define LOCK_INIT(p) InitializeCriticalSection(p)
#define LOCK(p) EnterCriticalSection(p)
#define UNLOCK(p) LeaveCriticalSection(p)
#define LOCK_CLOSE(p) DeleteCriticalSection(p)
#else
#include <pthread.h>
typedef pthread_mutex_t queue_lock;
#define LOCK_INIT(p) (void)pthread_mutex_init(p, NULL)
#define LOCK(p) (void)pthread_mutex_lock(p)
#define UNLOCK(p) (void)pthread_mutex_unlock(p)
#define LOCK_CLOSE(p) (void)pthread_mutex_destroy(p)
#endif

/* All strings are borrowed until this synchronous callback returns. */
struct sip_event {
    int kind, code;
    uint32_t account_id;
    uint64_t call_generation;
    const char *caller_name, *caller_number, *detail;
};
typedef void (*event_callback)(const struct sip_event *event, void *arg);
enum { EV_READY=1, EV_REGISTERING, EV_REGISTERED, EV_REG_FAILED,
       EV_CALLING, EV_RINGING, EV_INCOMING, EV_ACTIVE, EV_CLOSED, EV_ERROR, EV_STOPPED, EV_STALE, EV_REMOVED, EV_REMOVE_FAILED, EV_ACCOUNT_APPLIED, EV_ACCOUNT_REJECTED, EV_AUDIO_APPLIED, EV_AUDIO_FAILED, EV_RING_STARTED, EV_RING_STOPPED, EV_ANSWER_FAILED, EV_BUSY_INCOMING, EV_DIAL_STARTED, EV_DIAL_FAILED, EV_LOCAL_END, EV_INTERRUPTING,
       EV_PROGRESS, EV_HOLD, EV_RESUME, EV_DTMF_START, EV_DTMF_END, EV_TRANSFER_FAILED, EV_REDIRECT, EV_ANSWER_CLOSED, EV_TEST_STARTED, EV_TEST_LEVEL, EV_TEST_STOPPED, EV_TEST_FAILED, EV_SELECTED, EV_MUTED, EV_LOCAL_HOLD_PENDING, EV_LOCAL_HOLD_DONE, EV_CONTROL_FAILED, EV_TRANSFER_PENDING, EV_TRANSFER_RESULT, EV_CONTROL_REJECTED, EV_RECOVERY_FAILED, EV_RECOVERING, EV_CONSULT_FAILED, EV_CALL_ERROR, EV_SECURITY };
enum { OP_ACCOUNT=1, OP_DIAL, OP_ANSWER, OP_HANGUP, OP_STOP, OP_REMOVE, OP_SOUND, OP_PREVIEW, OP_PREVIEW_STOP, OP_SOUND_RESTORE, OP_REJECT, OP_TEST_MIC, OP_TEST_SPEAKER, OP_TEST_STOP, OP_DISABLE, OP_ENABLE, OP_MUTE, OP_HOLD, OP_TRANSFER, OP_CONSULT, OP_SWITCH, OP_CANCEL_CONSULT, OP_COMPLETE_TRANSFER };

struct request {
    struct request *next;
    int operation;
    uint32_t account_id;
    uint64_t call_generation;
    char *registrar, *username, *password, *destination, *auth_username, *outbound_proxy;
    int transport, media_security;
};
struct sip_account {
    uint32_t id;
    struct ua *ua;
    char registrar[256], next_hop[256];
    int transport, media_security;
};
struct native_dialog {
    struct call *call;
    struct ua *ua;
    bool answered, muted, consultation, hold_target, return_held, local_ending;
    uint32_t account_id;
    uint64_t generation, deadline;
    int pending, recovery;
};
struct engine {
    struct mqueue *queue;
    queue_lock lock;
    bool running;
    bool stop_requested;
    bool stopping;
    struct tmr shutdown_timer;
    struct tmr network_timer;
    struct request *pending;
    struct sip_account accounts[8];
    struct native_dialog dialogs[2];
    struct native_dialog *current, *selected, *action_target;
    struct tmr control_timer;
    int control_action;
    char consultation_destination[65];
    char test_ca[1024], test_dns_ip[64];
    uint16_t test_dns_port;
    char recording[128];
    int audio_mode;
    bool ringtone_enabled;
    int ringtone_variant;
    unsigned ringtone_volume;
    uint64_t test_id, test_deadline;
    int test_kind;
    struct ausrc_st *test_source;
    struct play *test_player;
    struct tmr test_timer;
    RE_ATOMIC unsigned test_peak;
    RE_ATOMIC int test_error;
    bool preview;
    bool platform_audio_loaded;
    bool diagnostics;
    uint64_t diagnostic_started;
    char input_device[128], output_device[128], ring_device[128];
    struct play *ring;
    event_callback callback;
    void *callback_arg;
};

/* The Rust lease permits one native event loop; libre's trace callback uses
 * the upstream UA argument, so keep this thread-owned diagnostic context here. */
static struct engine *diagnostic_engine;
/* The Rust lease serializes engines; tokens remain unique across recreation. */
static uint64_t next_call_generation;

/* This engine's queue lock also serializes token reservations from Rust. */
uint64_t sip_engine_reserve_call_generation(void *handle) {
    struct engine *e = handle;
    LOCK(&e->lock);
    uint64_t generation = ++next_call_generation;
    UNLOCK(&e->lock);
    return generation;
}

static const char *diagnostic_scope(const struct sa *address) {
    if (!address || !sa_isset(address, SA_ADDR)) return "unknown";
    if (sa_is_loopback(address)) return "loopback";
    if (sa_af(address) == AF_INET6) return "ipv6";
    if (sa_af(address) != AF_INET) return "unknown";
    uint32_t ip = sa_in(address);
    /* RFC 1918 and RFC 6598, reported as categories, never actual addresses. */
    if ((ip & 0xff000000u) == 0x0a000000u ||
        (ip & 0xfff00000u) == 0xac100000u ||
        (ip & 0xffff0000u) == 0xc0a80000u) return "private";
    if ((ip & 0xffc00000u) == 0x64400000u) return "shared";
    return "other-ipv4";
}

static const char *diagnostic_method(const struct pl *method) {
    const char *names[] = { "INVITE", "ACK", "BYE", "CANCEL", "REGISTER", "OPTIONS", "UPDATE", "PRACK" };
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); ++i)
        if (!pl_strcmp(method, names[i])) return names[i];
    return "other";
}

static bool diagnostic_contact_matches(const struct sip_hdr *header, const struct sip_msg *message, void *arg) {
    struct sip_addr contact;
    struct sa address;
    struct pl expires;
    (void)message;
    if (sip_addr_decode(&contact, &header->val) ||
        pl_strcasecmp(&contact.uri.scheme, "sip") ||
        sa_set(&address, &contact.uri.host, sip_transp_port(SIP_TRANSP_UDP, contact.uri.port)))
        return false;
    if (!msg_param_decode(&contact.params, "expires", &expires) && !pl_u32(&expires))
        return false;
    return sa_cmp(&address, arg, SA_ALL);
}

static void diagnostic_packet(struct engine *e, bool tx, const struct sa *src, const struct sa *dst,
                              const uint8_t *packet, size_t length) {
    struct sip_msg *message = NULL;
    struct mbuf *buffer;
    const char *local = "unknown", *contact_scope = "none", *contact_match = "unknown";
    /* Only SIP/UDP is enabled. Bound the diagnostic copy to one datagram. */
    if (length > 65535) return;
    buffer = mbuf_alloc(length);
    if (!buffer) return;
    int error = mbuf_write_mem(buffer, packet, length);
    buffer->pos = 0;
    if (!error) error = sip_msg_decode(&message, buffer);
    if (error) { mem_deref(buffer); return; }
    const char *method = diagnostic_method(message->req ? &message->met : &message->cseq.met);
#ifdef __APPLE__
    const struct sa *own = tx ? src : dst;
    char interface_name[256];
    if (!net_if_getname(interface_name, sizeof(interface_name), sa_af(own), own))
        local = !strncmp(interface_name, "utun", 4) ? "tunnel" : "other";
#endif
    /* INVITE Contact denotes the sender's target, including on a 2xx response.
       A different target is legal SIP; this comparison is evidence, not a fix. */
    if (!strcmp(method, "INVITE")) {
        const struct sip_hdr *header = sip_msg_hdr(message, SIP_HDR_CONTACT);
        if (header) {
            struct sip_addr contact;
            struct sa address;
            contact_scope = "unknown";
            if (!sip_addr_decode(&contact, &header->val) &&
                !pl_strcasecmp(&contact.uri.scheme, "sip") &&
                !sa_set(&address, &contact.uri.host, sip_transp_port(SIP_TRANSP_UDP, contact.uri.port))) {
                contact_scope = diagnostic_scope(&address);
                contact_match = sa_cmp(&address, src, SA_ALL) ? "yes" : "no";
            }
        }
    }
    unsigned status = message->scode >= 100 && message->scode <= 699 ? message->scode : 0;
    fprintf(stderr, "sip.diag packet=%s status=%u method=%s local=%s peer=%s contact=%s contact-match=%s ms=%" PRIu64 " call=%" PRIu64 "\n",
            tx ? "tx" : "rx", status, method, local, diagnostic_scope(tx ? dst : src), contact_scope, contact_match, tmr_jiffies() - e->diagnostic_started, e->current->generation);
    if (!tx && status >= 200 && status < 300 && !strcmp(method, "REGISTER")) {
        struct pl received = {0}, rport = {0};
        bool has_received = !msg_param_exists(&message->via.params, "received", &received);
        bool valued_received = !msg_param_decode(&message->via.params, "received", &received);
        bool has_rport = !msg_param_exists(&message->via.params, "rport", &rport);
        bool valued_rport = !msg_param_decode(&message->via.params, "rport", &rport);
        /* Presence only: never export the Via host, source port or raw parameters. */
        fprintf(stderr, "sip.diag register-via received=%s rport=%s ms=%" PRIu64 " call=%" PRIu64 "\n",
                has_received ? "present" : "missing",
                !has_rport ? "missing" : valued_rport ? "valued" : "empty",
                tmr_jiffies() - e->diagnostic_started, e->current->generation);
        struct sa mapped = message->via.addr;
        const char *target = "unknown";
        bool numeric_port = valued_rport && rport.l && rport.l <= 5;
        for (size_t i = 0; numeric_port && i < rport.l; ++i)
            if (rport.p[i] < '0' || rport.p[i] > '9') numeric_port = false;
        uint32_t port = numeric_port ? pl_u32(&rport) : 0;
        if (port && port <= 65535 &&
            (!has_received || (valued_received && !sa_set(&mapped, &received, (uint16_t)port))) &&
            sa_isset(&mapped, SA_ADDR)) {
            sa_set_port(&mapped, (uint16_t)port);
            target = sip_msg_hdr_apply(message, true, SIP_HDR_CONTACT,
                                      diagnostic_contact_matches, &mapped) ? "present" : "missing";
        }
        uint32_t contacts = sip_msg_hdr_count(message, SIP_HDR_CONTACT);
        fprintf(stderr, "sip.diag register-contacts count=%s mapped-target=%s ms=%" PRIu64 " call=%" PRIu64 "\n",
                !contacts ? "none" : contacts == 1 ? "one" : "many", target,
                tmr_jiffies() - e->diagnostic_started, e->current->generation);
    }
    mem_deref(message);
    mem_deref(buffer);
}

/* Opt-in, allowlisted markers only. Never print packet bytes, URIs or addresses. */
static void diagnostic_trace(bool tx, enum sip_transp tp, const struct sa *src,
                             const struct sa *dst, const uint8_t *packet,
                             size_t length, void *arg) {
    const char *name = NULL;
    (void)tp; (void)arg;
    struct engine *e = diagnostic_engine;
    if (!e) return;
    if (length >= 4 && !memcmp(packet, "BYE ", 4)) name = "BYE";
    else if (length >= 7 && !memcmp(packet, "CANCEL ", 7)) name = "CANCEL";
    else if (length >= 4 && !memcmp(packet, "ACK ", 4)) name = "ACK";
    else if (length >= 7 && !memcmp(packet, "INVITE ", 7)) name = "INVITE";
    else if (length >= 8 && !memcmp(packet, "SIP/2.0 ", 8)) name = "response";
    if (name) {
        fprintf(stderr, "sip.diag %s=%s ms=%" PRIu64 " call=%" PRIu64 "\n", tx ? "tx" : "rx", name, tmr_jiffies() - e->diagnostic_started, e->current->generation);
        diagnostic_packet(e, tx, src, dst, packet, length);
    }
}

static void emit_account(struct engine *e, int kind, int code, uint32_t account_id) {
    if (e->diagnostics) {
        const char *name = kind == EV_ACTIVE ? "active" : (kind == EV_CLOSED || kind == EV_ANSWER_CLOSED) ? "closed" : NULL;
        if (name) fprintf(stderr, "sip.diag event=%s ms=%" PRIu64 " call=%" PRIu64 "\n", name, tmr_jiffies() - e->diagnostic_started, e->current->generation);
    }
    const struct sip_event event = { kind, code, account_id, e->current->generation, NULL, NULL, NULL };
    e->callback(&event, e->callback_arg);
}
static void emit(struct engine *e, int kind, int code) {
    if (kind == EV_LOCAL_END) e->current->local_ending = true;
    if (kind == EV_INTERRUPTING) e->stopping = true;
    emit_account(e, kind, code, e->current->account_id);
}
static struct sip_account *find_account(struct engine *e, uint32_t id) {
    for (size_t i = 0; i < 8; ++i) if (e->accounts[i].id == id && id) return &e->accounts[i];
    return NULL;
}
static const char *platform_audio(void) {
#ifdef __APPLE__
    return "coreaudio";
#elif defined(WIN32)
    return "wasapi";
#else
    return NULL;
#endif
}
static int ensure_platform_audio(struct engine *e) {
    if (e->platform_audio_loaded) return 0;
    const char *module = platform_audio();
    if (!module) return ENOTSUP;
    int error = module_load("", module);
    if (!error) e->platform_audio_loaded = true;
    return error;
}
static void ring_finished(struct play *play, void *arg) {
    struct engine *e = arg;
    (void)play;
    e->preview = false;
    emit(e, EV_RING_STOPPED, 0);
}
static void stop_ring(struct engine *e) {
    if (e->ring) e->ring = mem_deref(e->ring);
    e->preview = false;
}
static int start_ring(struct engine *e, const char *device, bool preview) {
    int error = sip_audio_device_available(device, 0);
    if (!error) error = ensure_platform_audio(e);
    if (error) return error;
    stop_ring(e);
    /* PCM is generated in RAM and played through the existing baresip player. */
    const uint32_t rate = 48000, samples = rate * 2;
    struct mbuf *tone = mbuf_alloc(samples * sizeof(int16_t));
    if (!tone) return ENOMEM;
    for (uint32_t i = 0; i < samples; ++i) {
        int16_t sample = sip_ringtone_sample(e->ringtone_variant, e->ringtone_volume, i);
        (void)mbuf_write_mem(tone, (const uint8_t *)&sample, sizeof(sample));
    }
    tone->pos = 0;
    error = play_tone(&e->ring, baresip_player(), tone, rate, 1, preview ? 1 : -1, platform_audio(), *device ? device : "default");
    mem_deref(tone);
    if (!error) {
        e->preview = preview;
        play_set_finish_handler(e->ring, ring_finished, e);
        emit(e, EV_RING_STARTED, 0);
    }
    return error;
}
/* Only capture callbacks touch atomics. All ownership, events and frees stay on
 * the existing libre loop; source destructors stop/join native callbacks. */
static void emit_test(struct engine *e, int kind, int code, uint64_t id) {
    const struct sip_event event = { kind, code, (uint32_t)e->test_kind, id, NULL, NULL, NULL };
    e->callback(&event, e->callback_arg);
}
static void stop_audio_test(struct engine *e) {
    uint64_t id = e->test_id;
    if (!id) return;
    tmr_cancel(&e->test_timer);
    e->test_source = mem_deref(e->test_source);
    e->test_player = mem_deref(e->test_player);
    e->test_id = 0;
    re_atomic_rlx_set(&e->test_peak, 0);
    if (id) emit_test(e, EV_TEST_STOPPED, 0, id);
}
static void test_read(struct auframe *frame, void *arg) {
    struct engine *e = arg;
    if (frame->fmt != AUFMT_S16LE) { re_atomic_rlx_set(&e->test_error, ENOTSUP); return; }
    unsigned peak = sip_audio_peak_s16(frame->sampv, frame->sampc);
    unsigned previous = re_atomic_rlx(&e->test_peak);
    while (previous < peak && !re_atomic_compare_exchange_weak(&e->test_peak, &previous, peak, re_memory_order_relaxed, re_memory_order_relaxed)) {}
}
static void test_source_error(int error, const char *text, void *arg) {
    struct engine *e = arg;
    (void)text;
    re_atomic_rlx_set(&e->test_error, error ? error : EIO);
}
static void test_tick(void *arg) {
    struct engine *e = arg;
    int error = re_atomic_exchange(&e->test_error, 0, re_memory_order_relaxed);
    if (error) { emit_test(e, EV_TEST_FAILED, error, e->test_id); stop_audio_test(e); return; }
    if (tmr_jiffies() >= e->test_deadline) { stop_audio_test(e); return; }
    if (e->test_kind == 0) emit_test(e, EV_TEST_LEVEL, (int)re_atomic_exchange(&e->test_peak, 0, re_memory_order_relaxed), e->test_id);
    tmr_start(&e->test_timer, 100, test_tick, e);
}
static int start_audio_test(struct engine *e, const struct request *r) {
    if (e->current->call || e->test_id || e->preview) return EBUSY;
    if (!r->call_generation || strlen(r->registrar) > 127) return EINVAL;
    bool microphone = r->operation == OP_TEST_MIC;
    int error = 0;
    if (!e->audio_mode) {
        error = sip_audio_device_available(r->registrar, microphone);
        if (!error) error = ensure_platform_audio(e);
        if (error) return error;
    }
    e->test_kind = microphone ? 0 : 1;
    re_atomic_rlx_set(&e->test_peak, 0);
    re_atomic_rlx_set(&e->test_error, 0);
    if (microphone) {
        struct ausrc_prm parameters = { .srate=48000, .ch=1, .ptime=20, .fmt=AUFMT_S16LE };
        error = ausrc_alloc(&e->test_source, baresip_ausrcl(), e->audio_mode ? "ausine" : platform_audio(),
            &parameters, e->audio_mode ? "440" : *r->registrar ? r->registrar : "default", test_read, test_source_error, e);
    } else {
        const uint32_t rate = 48000, samples = rate * 2;
        struct mbuf *tone = mbuf_alloc(samples * sizeof(int16_t));
        if (!tone) return ENOMEM;
        for (uint32_t i = 0; i < samples; ++i) {
            double seconds = (double)i / rate;
            double fade = fmin(1.0, fmin(seconds / 0.01, (2.0 - seconds) / 0.01));
            int16_t sample = (int16_t)(2200.0 * fade * sin(2.0 * 3.141592653589793 * 440 * seconds));
            (void)mbuf_write_mem(tone, (const uint8_t *)&sample, sizeof(sample));
        }
        tone->pos = 0;
        error = play_tone(&e->test_player, baresip_player(), tone, rate, 1, 1,
            e->audio_mode ? "aufile" : platform_audio(), e->audio_mode ? e->recording : *r->registrar ? r->registrar : "default");
        mem_deref(tone);
    }
    if (error) { e->test_source = mem_deref(e->test_source); e->test_player = mem_deref(e->test_player); return error; }
    e->test_id = r->call_generation;
    e->test_deadline = tmr_jiffies() + (microphone ? 10000 : 2000);
    emit_test(e, EV_TEST_STARTED, 0, e->test_id);
    tmr_start(&e->test_timer, 100, test_tick, e);
    return 0;
}
static int apply_sound(struct engine *e, const struct request *r) {
    if (e->current->call || e->test_id) return EBUSY;
    if (strlen(r->registrar) > 127 || strlen(r->username) > 127 || strlen(r->password) > 127) return EINVAL;
    if (r->operation != OP_SOUND_RESTORE) {
        int error = sip_audio_device_available(r->registrar, 1);
        if (!error) error = sip_audio_device_available(r->username, 0);
        if (!error) error = sip_audio_device_available(r->password, 0);
        if (error) return error;
    }
    unsigned enabled, volume;
    int variant;
    char trailing;
    if (sscanf(r->destination, "%u:%d:%u%c", &enabled, &variant, &volume, &trailing) != 3 ||
        enabled > 1 || variant < 0 || variant > 2 || volume > 100) return EINVAL;
    stop_ring(e);
    str_ncpy(e->input_device, r->registrar, sizeof(e->input_device));
    str_ncpy(e->output_device, r->username, sizeof(e->output_device));
    str_ncpy(e->ring_device, r->password, sizeof(e->ring_device));
    e->ringtone_enabled = enabled != 0;
    e->ringtone_variant = variant;
    e->ringtone_volume = volume;
    if (!e->audio_mode) {
        struct config_audio *audio = &conf_config()->audio;
        str_ncpy(audio->src_dev, *e->input_device ? e->input_device : "default", sizeof(audio->src_dev));
        str_ncpy(audio->play_dev, *e->output_device ? e->output_device : "default", sizeof(audio->play_dev));
    }
    return 0;
}
static char *copy_string(const char *s) {
    size_t size = strlen(s ? s : "") + 1;
    char *copy = malloc(size);
    if (copy) memcpy(copy, s ? s : "", size);
    return copy;
}
static void free_request(struct request *r) {
    if (!r) return;
    if (r->password) {
        volatile char *p = r->password;
        size_t n = strlen(r->password);
        while (n--) *p++ = 0;
    }
    free(r->auth_username); free(r->outbound_proxy); free(r->registrar); free(r->username); free(r->password); free(r->destination); free(r);
}
static void remove_pending(struct engine *e, struct request *r) {
    struct request **p = &e->pending;
    while (*p && *p != r) p = &(*p)->next;
    if (*p) *p = r->next;
}

/* Shutdown never depends on allocation or available pipe capacity. */
static void shutdown_tick(void *arg) {
    struct engine *e = arg;
    bool stop;
    LOCK(&e->lock); stop = e->stop_requested; UNLOCK(&e->lock);
    if (stop) { emit(e, EV_INTERRUPTING, 0); stop_audio_test(e); stop_ring(e); ua_stop_all(true); re_cancel(); }
    else tmr_start(&e->shutdown_timer, 50, shutdown_tick, e);
}

static void network_tick(void *arg) {
    struct engine *e = arg;
    (void)sip_network_poll(e->current->call != NULL);
    tmr_start(&e->network_timer, 5000, network_tick, e);
}

/* Only the From identity is exposed. Never consult asserted/network identities. */
static int caller_privacy(const struct pl *name, const struct pl *value, void *arg) {
    if (pl_strcasecmp(name, "Privacy")) return 0;
    size_t offset = 0;
    while (offset < value->l) {
        size_t start = offset;
        while (offset < value->l && value->p[offset] != ';' && value->p[offset] != ',') ++offset;
        struct pl token = { value->p + start, offset - start };
        while (token.l && (*token.p == ' ' || *token.p == '\t')) { ++token.p; --token.l; }
        while (token.l && (token.p[token.l - 1] == ' ' || token.p[token.l - 1] == '\t')) --token.l;
        if (!pl_strcasecmp(&token, "id")) *(bool *)arg = true;
        if (offset < value->l) ++offset;
    }
    return 0;
}

/* Keep the largest complete UTF-8 prefix within the fixed callback array. Rust
 * additionally validates UTF-8 and rejects control/bidi formatting characters. */
static void caller_copy(char *out, size_t capacity, const char *value) {
    if (!value) return;
    size_t length = strlen(value);
    size_t count = length < capacity - 1 ? length : capacity - 1;
    if (count < length)
        while (count && (((unsigned char)value[count] & 0xc0) == 0x80)) --count;
    memcpy(out, value, count);
    out[count] = '\0';
}

static void emit_incoming(struct engine *e, const struct call *call, int kind, uint32_t account_id, uint64_t generation) {
    char name[129] = {0}, number[81] = {0};
    const char *peer = call_peeruri(call);
    struct pl text;
    struct uri from;
    bool withheld = false;
    (void)custom_hdrs_apply(call_get_custom_hdrs(call), caller_privacy, &withheld);
    pl_set_str(&text, peer);
    if (peer && !uri_decode(&from, &text)) {
        bool sip = !pl_strcasecmp(&from.scheme, "sip") || !pl_strcasecmp(&from.scheme, "sips");
        bool tel = !pl_strcasecmp(&from.scheme, "tel");
        /* libre represents tel's subscriber in host, SIP's identity in user.
         * uri_decode excludes the password from user before the '@'. */
        const struct pl *user = sip ? &from.user : tel ? &from.host : NULL;
        withheld = withheld || !pl_strcasecmp(&from.host, "anonymous.invalid") ||
                   (user && !pl_strcasecmp(user, "anonymous"));
        if (!withheld && user && pl_isset(user)) {
            char decoded[321] = {0};
            int length = re_snprintf(decoded, sizeof(decoded), "%H", uri_user_unescape, user);
            if (length >= 0 && (size_t)length < sizeof(decoded) && !memchr(decoded, '\0', (size_t)length)) {
                struct pl identity;
                pl_set_str(&identity, decoded);
                withheld = !pl_strcasecmp(&identity, "anonymous");
                if (!withheld) caller_copy(number, sizeof(number), decoded);
            }
        }
    }
    caller_copy(name, sizeof(name), withheld ? "Unterdrückte Rufnummer" : call_peername(call));
    const struct sip_event event = { kind, 0, account_id, generation, name, number, NULL };
    e->callback(&event, e->callback_arg);
}

/* Only numeric status prefixes produced by baresip are copied; reason/target text never leaves C. */
static int event_status(const char *text, char separator) {
    if (!text || strlen(text) < 4 || text[0] < '3' || text[0] > '6' ||
        text[1] < '0' || text[1] > '9' || text[2] < '0' || text[2] > '9' || text[3] != separator)
        return 0;
    return (text[0] - '0') * 100 + (text[1] - '0') * 10 + text[2] - '0';
}

static const char *transport_name(int transport) { return transport == 2 ? "tls" : transport == 1 ? "tcp" : "udp"; }
enum { SECURITY_CERTIFICATE = SIP_SECURITY_CERTIFICATE, SECURITY_TRANSPORT = SIP_SECURITY_TRANSPORT, SECURITY_MEDIA = SIP_SECURITY_MEDIA };
static int security_error(const struct sip_account *account, int error) {
    if (error == SECURITY_MEDIA) return SECURITY_MEDIA;
    if (account && account->transport == 2 && error == EAUTH) return SECURITY_CERTIFICATE;
    if (account && account->transport == 2 && error) return SECURITY_TRANSPORT;
    return error;
}
static void emit_security(struct engine *e, const struct sip_account *account, struct call *call, bool negotiating) {
    if (!call || e->current->call != call) return;
    enum sip_transp tp = call_transp(call);
    int observed = tp == SIP_TRANSP_UDP ? 1 : tp == SIP_TRANSP_TCP ? 2 : tp == SIP_TRANSP_TLS ? 3 : 0;
    char host[256]; str_ncpy(host, account->next_hop, sizeof(host));
    char *port = strchr(host, ':'); if (port) *port = 0;
    if (tp == SIP_TRANSP_TLS && call_tls_peer_verified_for(call, host)) observed |= 4;
    if (!negotiating && observed) {
        if (!account->media_security) observed |= 1 << 3;
        else if (stream_is_secure(audio_strm(call_audio(call)))) observed |= 2 << 3;
    }
    emit(e, EV_SECURITY, observed);
}
#include "call_controls.h"

static void event_handler(enum bevent_ev kind, struct bevent *event, void *arg) {
    struct engine *e = arg;
    struct call *call = bevent_get_call(event);
    struct ua *ua = bevent_get_ua(event);
    struct sip_account *account = NULL;
    for (size_t i = 0; i < 8; ++i) if (ua && e->accounts[i].ua == ua) { account = &e->accounts[i]; break; }
    if (!account) return;
    if (kind != BEVENT_CALL_OUTGOING) e->current = e->selected;
    for (size_t i = 0; call && i < 2; ++i)
        if (e->dialogs[i].call == call) e->current = &e->dialogs[i];
    switch (kind) {
    case BEVENT_REGISTERING: emit_account(e, EV_REGISTERING, 0, account->id); break;
    case BEVENT_REGISTER_OK: emit_account(e, EV_REGISTERED, 0, account->id); break;
    case BEVENT_REGISTER_FAIL: emit_account(e, EV_REG_FAILED, ua_registration_error(ua) ? security_error(account, ua_registration_error(ua)) : event_status(bevent_get_text(event), ' '), account->id); break;
    case BEVENT_CALL_INCOMING:
        if (account->transport == 2 && call_transp(call) != SIP_TRANSP_TLS) { ua_hangup(ua, call, 488, "TLS Required"); break; }
        stop_audio_test(e);
        if (e->current->call && e->current->call != call) {
            emit_incoming(e, call, EV_BUSY_INCOMING, account->id, sip_engine_reserve_call_generation(e));
            ua_hangup(ua, call, 486, "Busy Here");
            break;
        }
        if (e->current->call != call) { e->current->generation = sip_engine_reserve_call_generation(e); e->current->answered = false; }
        e->current->call = call; e->current->ua = ua; e->current->account_id = account->id;
        call_set_control_handler(call, control_result, e);
        emit_incoming(e, call, EV_INCOMING, account->id, e->current->generation);
        stop_ring(e);
        if (!e->audio_mode && e->ringtone_enabled) {
            int error = start_ring(e, e->ring_device, false);
            if (error) emit(e, EV_ERROR, error);
        }
        break;
    case BEVENT_CALL_OUTGOING:
        stop_audio_test(e);
        stop_ring(e);
        e->current->answered = false; e->current->call = call; e->current->ua = ua; e->current->account_id = account->id;
        call_set_control_handler(call, control_result, e);
        emit(e, EV_CALLING, 0); break;
    case BEVENT_CALL_RINGING: if (e->current->call == call) emit(e, EV_RINGING, 0); break;
    case BEVENT_CALL_PROGRESS: if (e->current->call == call) emit(e, EV_PROGRESS, 0); break;
    case BEVENT_CALL_HOLD: if (e->current->call == call) emit(e, EV_HOLD, 0); break;
    case BEVENT_CALL_RESUME:
        if (e->current->call == call) {
            /* Remote hold pauses receive checks; grant a fresh interval on
               resume before the first returning RTP packet can arrive. */
            stream_enable_rtp_timeout(audio_strm(call_audio(call)), conf_config()->avt.rtp_timeout * 1000);
            emit(e, EV_RESUME, 0);
        }
        break;
    case BEVENT_CALL_DTMF_START:
        if (e->current->call == call) {
            const char *text = bevent_get_text(event);
            if (text && text[0] && strchr("0123456789*#ABCD", text[0]) && !text[1]) {
                const char digit[2] = { text[0], 0 };
                const struct sip_event signal = { EV_DTMF_START, 0, account->id, e->current->generation, NULL, NULL, digit };
                e->callback(&signal, e->callback_arg);
            }
        }
        break;
    case BEVENT_CALL_DTMF_END: if (e->current->call == call) emit(e, EV_DTMF_END, 0); break;
    case BEVENT_CALL_TRANSFER:
        /* Defensive handling: accounts never advertise or accept transfers. */
        if (e->current->call == call) {
            (void)call_notify_sipfrag(call, 501, "Not Implemented");
            emit(e, EV_TRANSFER_FAILED, 501);
        }
        break;
    case BEVENT_CALL_TRANSFER_FAILED:
        if (e->current->call == call) emit(e, EV_TRANSFER_FAILED, event_status(bevent_get_text(event), ' '));
        break;
    case BEVENT_CALL_REDIRECT:
        if (e->current->call == call) emit(e, EV_REDIRECT, event_status(bevent_get_text(event), ','));
        break;
    case BEVENT_CALL_ESTABLISHED: if (e->current->call == call) { e->current->answered = true; stop_ring(e); emit(e, EV_ACTIVE, 0); } break;
    case BEVENT_CALL_CLOSED:
        if (e->current->call == call) {
            /* A sent answer without ACK is a failed setup, unlike an unanswered
             * remote CANCEL. Local hangup first marks the call terminated. */
            bool answer_failed = e->current->answered && call_state(call) == CALL_STATE_INCOMING && !call_scode(call);
            if (call_native_error(call)) emit(e, EV_CALL_ERROR, security_error(account, call_native_error(call)));
            stop_ring(e); e->current->call = NULL; e->current->ua = NULL;
            emit(e, answer_failed ? EV_ANSWER_CLOSED : EV_CLOSED, call_scode(call));
            e->current->account_id = 0;
            dialog_closed(e, call_scode(call));
        }
        break;
    case BEVENT_CALL_REMOTE_SDP:
    case BEVENT_CALL_LOCAL_SDP:
        emit_security(e, account, call, true);
        break;
    case BEVENT_CALL_MENC:
        emit_security(e, account, call, false);
        break;
    case BEVENT_AUDIO_ERROR:
        if (e->diagnostics) fprintf(stderr, "sip.diag event=audio-error ms=%" PRIu64 " call=%" PRIu64 "\n", tmr_jiffies() - e->diagnostic_started, e->current->generation);
        if (e->current->call == call) emit(e, EV_CALL_ERROR, EIO);
        break;
    default: break;
    }
    if (call && e->current->call == call && (kind == BEVENT_CALL_INCOMING || kind == BEVENT_CALL_ESTABLISHED || kind == BEVENT_CALL_PROGRESS || kind == BEVENT_CALL_OUTGOING)) emit_security(e, account, call, false);
    e->current = e->selected;
}

static void queue_handler(int id, void *data, void *arg) {
    struct engine *e = arg;
    struct request *r = data;
    int error = 0;
    uint64_t dial_generation = 0;
    char uri[512];
    struct sip_account *account = find_account(e, r->account_id);
    (void)id;
    e->current = e->selected;
    LOCK(&e->lock); remove_pending(e, r); UNLOCK(&e->lock);
    if (r->operation >= OP_MUTE && r->operation <= OP_COMPLETE_TRANSFER) {
        control_request(e, r);
        e->current = e->selected;
        free_request(r);
        return;
    }
    if (r->operation == OP_HANGUP) {
        for (size_t i = 0; i < 2; ++i)
            if (e->dialogs[i].generation == r->call_generation) e->current = &e->dialogs[i];
    }
    if ((r->operation == OP_ANSWER || r->operation == OP_HANGUP || r->operation == OP_REJECT)
        && r->call_generation != e->current->generation) {
        emit(e, EV_STALE, 0);
        free_request(r);
        return;
    }
    switch (r->operation) {
    case OP_ACCOUNT:
    case OP_ENABLE: {
        struct ua *replacement = NULL;
        if (e->current->call) { error = EBUSY; break; }
        if (!account) {
            for (size_t i = 0; i < 8; ++i) if (!e->accounts[i].id) { account = &e->accounts[i]; break; }
        }
        if (!account) { error = ENOSPC; break; }
        if (r->operation == OP_ACCOUNT && !strcmp(r->destination, "0")) {
            struct ua *previous = account->ua;
            account->ua = NULL;
            account->id = r->account_id;
            str_ncpy(account->registrar, r->registrar, sizeof(account->registrar));
            mem_deref(previous);
            emit_account(e, EV_ACCOUNT_APPLIED, 0, r->account_id);
            break;
        }
        (void)snprintf(uri, sizeof(uri), "<sip:%s@%s;transport=%s>;regint=60;audio_codecs=PCMU/8000/1,PCMA/8000/1", r->username, r->registrar, transport_name(r->transport));
        error = ua_alloc(&replacement, uri);
        if (!error) error = account_set_call_transfer(ua_account(replacement), false);
        if (!error) {
            ua_remove_extension(replacement, "norefersub");
            ua_remove_extension(replacement, "replaces");
        }
        if (!error) error = ua_add_xhdr_filter(replacement, "Privacy");
        if (!error) error = account_set_auth_user(ua_account(replacement), r->auth_username[0] ? r->auth_username : r->username);
        if (!error) error = account_set_auth_pass(ua_account(replacement), r->password);
        if (!error && r->media_security) error = account_set_mediaenc(ua_account(replacement), "srtp-mand");
        if (!error && r->outbound_proxy[0]) {
            char proxy[320];
            snprintf(proxy, sizeof(proxy), "sip:%s;transport=%s", r->outbound_proxy, transport_name(r->transport));
            error = account_set_outbound(ua_account(replacement), proxy, 0);
        }
        if (error) { mem_deref(replacement); break; }
        struct ua *previous = account->ua;
        struct sip_account previous_config = *account;
        account->ua = NULL;
        account->id = r->account_id;
        account->ua = replacement;
        account->transport = r->transport; account->media_security = r->media_security;
        str_ncpy(account->next_hop, r->outbound_proxy[0] ? r->outbound_proxy : r->registrar, sizeof(account->next_hop));
        str_ncpy(account->registrar, r->registrar, sizeof(account->registrar));
        error = ua_register(replacement);
        if (error) {
            *account = previous_config;
            mem_deref(replacement);
            if (!previous) memset(account, 0, sizeof(*account));
        } else {
            /* The UA destructor retires pending REGISTER before unregistering.
               An explicit ua_unregister would overlap the owned request. */
            mem_deref(previous);
            emit_account(e, EV_ACCOUNT_APPLIED, 0, r->account_id);
        }
        break;
    }
    case OP_DISABLE:
        if (e->current->call) { error = EBUSY; break; }
        if (account) {
            struct ua *previous = account->ua;
            account->ua = NULL;
            mem_deref(previous);
        }
        emit_account(e, EV_ACCOUNT_APPLIED, 0, r->account_id);
        break;
    case OP_REMOVE:
        if (e->current->call) { emit_account(e, EV_REMOVE_FAILED, EBUSY, r->account_id); break; }
        if (account) {
            struct ua *previous = account->ua;
            memset(account, 0, sizeof(*account));
            mem_deref(previous);
        }
        emit_account(e, EV_REMOVED, 0, r->account_id);
        break;
    case OP_DIAL: {
        char destination[81] = {0};
        dial_generation = r->call_generation;
        caller_copy(destination, sizeof(destination), r->destination);
        const struct sip_event started = { EV_DIAL_STARTED, 0, r->account_id, dial_generation, NULL, destination, NULL };
        e->callback(&started, e->callback_arg);
        if (e->current->call) { error = EBUSY; break; }
        stop_audio_test(e);
        e->current->generation = dial_generation;
        e->current->account_id = r->account_id;
        if (!e->audio_mode) {
            error = sip_audio_device_available(e->input_device, 1);
            if (!error) error = sip_audio_device_available(e->output_device, 0);
            if (error) break;
        }
        if (!account || !account->ua || !ua_isregistered(account->ua)) { error = ENOTCONN; break; }
        (void)snprintf(uri, sizeof(uri), "sip:%s@%s;transport=%s", r->destination, account->registrar, transport_name(account->transport));
        error = ua_connect(account->ua, &e->current->call, NULL, uri, VIDMODE_OFF);
        /* ua_connect destroys a partially-created call when setup fails. */
        if (error) { e->current->call = NULL; e->current->ua = NULL; }
        break;
    }
    case OP_ANSWER:
        /* A queued hangup or remote cancellation may have already closed this
         * generation. Report a stale action, rather than an audio/native error. */
        if (!e->current->call) { emit(e, EV_STALE, 0); break; }
        if (!e->audio_mode) {
            error = sip_audio_device_available(e->input_device, 1);
            if (!error) error = sip_audio_device_available(e->output_device, 0);
            if (error) break;
        }
        error = e->current->call ? ua_answer(e->current->ua, e->current->call, VIDMODE_OFF) : EINVAL;
        if (!error) { e->current->answered = true; stop_ring(e); }
        break;
    case OP_REJECT:
        if (!e->current->call || e->current->answered || call_state(e->current->call) != CALL_STATE_INCOMING) {
            emit(e, EV_STALE, 0);
            break;
        }
        stop_ring(e);
        emit(e, EV_LOCAL_END, 0);
        ua_hangup(e->current->ua, e->current->call, 486, "Busy Here");
        break;
    case OP_HANGUP:
        if (e->current->consultation) {
            struct native_dialog *parent = e->current == &e->dialogs[0] ? &e->dialogs[1] : &e->dialogs[0];
            parent->return_held = false;
        }
        stop_ring(e);
        /* A second already-queued hangup of this generation is idempotent.
         * A different live call was rejected by the generation guard above. */
        if (e->current->call) { emit(e, EV_LOCAL_END, 0); ua_hangup(e->current->ua, e->current->call, 0, NULL); }
        break;
    case OP_SOUND:
    case OP_SOUND_RESTORE:
        error = apply_sound(e, r);
        emit(e, error ? EV_AUDIO_FAILED : EV_AUDIO_APPLIED, error);
        break;
    case OP_PREVIEW:
        if (e->current->call || e->test_id) error = EBUSY;
        else if (strlen(r->registrar) > 127) error = EINVAL;
        else {
            error = sip_audio_device_available(r->registrar, 0);
            if (!error) error = start_ring(e, r->registrar, true);
        }
        emit(e, error ? EV_AUDIO_FAILED : EV_AUDIO_APPLIED, error);
        break;
    case OP_PREVIEW_STOP:
        if (e->preview) stop_ring(e);
        emit(e, EV_AUDIO_APPLIED, 0);
        break;
    case OP_TEST_MIC:
    case OP_TEST_SPEAKER:
        error = start_audio_test(e, r);
        emit(e, error ? EV_AUDIO_FAILED : EV_AUDIO_APPLIED, error);
        if (error) emit_test(e, EV_TEST_FAILED, error, r->call_generation);
        break;
    case OP_TEST_STOP:
        if (e->test_id == r->call_generation) stop_audio_test(e);
        emit(e, EV_AUDIO_APPLIED, 0);
        break;
    case OP_STOP:
        emit(e, EV_INTERRUPTING, 0);
        stop_audio_test(e);
        stop_ring(e);
        ua_stop_all(true);
        re_cancel();
        break;
    default: error = EINVAL;
    }
    if (error && (r->operation == OP_ACCOUNT || r->operation == OP_DISABLE || r->operation == OP_ENABLE)) emit_account(e, EV_ACCOUNT_REJECTED, error, r->account_id);
    if (error && r->operation == OP_DIAL) {
        const struct sip_event failed = { EV_DIAL_FAILED, error, r->account_id, dial_generation, NULL, NULL, NULL };
        e->callback(&failed, e->callback_arg);
        if (!e->current->call) { e->current->ua = NULL; e->current->account_id = 0; }
    }
    /* Dial failures already carry the attempted generation above. A generic
     * error would attach that failed attempt to an incoming call that won. */
    if (error && r->operation != OP_DIAL && r->operation != OP_TEST_MIC && r->operation != OP_TEST_SPEAKER) emit(e, r->operation == OP_ANSWER ? EV_ANSWER_FAILED : EV_ERROR, error);
    free_request(r);
}

void *sip_engine_create(event_callback callback, void *arg, int audio_mode, const char *recording) {
    struct engine *e = calloc(1, sizeof(*e));
    if (!e || !callback) { free(e); return NULL; }
    e->current = e->selected = &e->dialogs[0];
    LOCK_INIT(&e->lock);
    e->callback = callback; e->callback_arg = arg; e->audio_mode = audio_mode; e->ringtone_enabled = true; e->ringtone_volume = 100;
    re_atomic_rlx_set(&e->test_peak, 0); re_atomic_rlx_set(&e->test_error, 0);
    const char *diagnostics = getenv("SIP_DIAGNOSTICS");
    e->diagnostics = diagnostics && !strcmp(diagnostics, "1");
    str_ncpy(e->recording, recording ? recording : "", sizeof(e->recording));
    return e;
}

int sip_engine_run(void *handle) {
    struct engine *e = handle;
    /* Create incoming call objects without the upstream menu module. This does
       not answer them: the account defaults to manual answer mode. */
    const char config[] =
        "sip_listen 0.0.0.0:0\n"
        "sip_transports udp,tcp,tls\n"
        "sip_tls_resume none\n"
        "sip_verify_server yes\n"
        "call_max_calls 2\n"
        "call_accept yes\n"
        "rtp_ports 40000-40100\n"
        /* Release an established sendrecv call after sustained media loss.
           Upstream suspends this watchdog while the peer holds the call. */
        "rtp_timeout 60\n"
        /* Two 20 ms packets on stable paths; retain headroom for VPN jitter. */
        "audio_jitter_buffer_type adaptive\n"
        "audio_jitter_buffer_ms 40-200\n";
    struct config_audio *audio;
    log_level_set(LEVEL_WARN);
    log_enable_color(false);
    int error = libre_init();
    bool initialized = !error;
    if (!error) error = conf_configure_buf((const uint8_t *)config, sizeof(config)-1);
    if (!error) error = re_thread_async_init(2);
    audio = &conf_config()->audio;
    if (e->audio_mode) {
        str_ncpy(audio->src_mod, "ausine", sizeof(audio->src_mod));
        str_ncpy(audio->src_dev, "440", sizeof(audio->src_dev));
        str_ncpy(audio->play_mod, "aufile", sizeof(audio->play_mod));
        str_ncpy(audio->play_dev, e->recording, sizeof(audio->play_dev));
    } else {
#ifdef __APPLE__
        str_ncpy(audio->src_mod, "coreaudio", sizeof(audio->src_mod));
        str_ncpy(audio->play_mod, "coreaudio", sizeof(audio->play_mod));
#elif defined(WIN32)
        str_ncpy(audio->src_mod, "wasapi", sizeof(audio->src_mod));
        str_ncpy(audio->play_mod, "wasapi", sizeof(audio->play_mod));
#else
        error = ENOTSUP;
#endif
    }
    if (!e->audio_mode) {
        str_ncpy(audio->src_dev, "default", sizeof(audio->src_dev));
        str_ncpy(audio->play_dev, "default", sizeof(audio->play_dev));
    }
    if (!error) error = baresip_init(conf_config());
#ifdef SIP_NATIVE_TEST_HOOKS
    if (!error && e->test_dns_port) {
        struct sa server;
        error = sa_set_str(&server,e->test_dns_ip,e->test_dns_port);
        if (!error) error = dnsc_srv_set(net_dnsc(baresip_network()),&server,1);
    }
#endif
    /* Upstream auto-enumeration excludes IPv4 loopback. Local PBX calls still
       need a matching media address; add it through the existing network API. */
    if (!error) {
        struct sa loopback;
        error = sa_set_str(&loopback, "127.0.0.1", 0);
        if (!error) error = net_add_address(baresip_network(), &loopback);
    }
    if (!error) error = ua_init("SIP native milestone", true, true, true);
    if (!error) error = tls_set_session_reuse(uag_tls(), false);
    if (!error) error = sip_security_configure(uag_tls(), e->test_ca[0] ? e->test_ca : NULL);
    if (!error && e->diagnostics) {
        e->diagnostic_started = tmr_jiffies();
        diagnostic_engine = e;
        sip_set_trace_handler(uag_sip(), diagnostic_trace);
    }
    if (!error) error = bevent_register(event_handler, e);
    if (!error) error = module_load("", "srtp");
    if (!error) error = module_load("", "g711");
    if (!error) error = module_load("", "auconv");
    if (!error) error = module_load("", "auresamp");
    if (!error) error = module_load("", "ausine");
    if (!error) error = module_load("", "aufile");
    if (!error && !e->audio_mode) error = ensure_platform_audio(e);
    LOCK(&e->lock);
    if (!error) error = mqueue_alloc(&e->queue, queue_handler, e);
    e->running = !error;
    UNLOCK(&e->lock);
    emit(e, EV_READY, error);
    if (!error) {
        tmr_start(&e->control_timer, 20, control_tick, e);
        tmr_start(&e->shutdown_timer, 50, shutdown_tick, e);
        sip_network_reset();
        tmr_start(&e->network_timer, 5000, network_tick, e);
        error = re_main(NULL);
    }
    tmr_cancel(&e->control_timer);
    tmr_cancel(&e->shutdown_timer);
    tmr_cancel(&e->network_timer);
    sip_network_reset();
    LOCK(&e->lock);
    e->running = false;
    e->queue = mem_deref(e->queue);
    while (e->pending) { struct request *r = e->pending; e->pending = r->next; free_request(r); }
    UNLOCK(&e->lock);
    if (initialized) {
        diagnostic_engine = NULL;
        stop_audio_test(e);
        stop_ring(e);
        bevent_unregister(event_handler);
        e->current->call = NULL; e->current->ua = NULL;
        memset(e->accounts, 0, sizeof(e->accounts));
        ua_close();
        module_app_unload();
        baresip_close();
        mod_close();
        conf_close();
        re_thread_async_close();
        libre_close();
    }
    emit(e, EV_STOPPED, error);
    return error;
}

int sip_engine_request(void *handle, int operation, uint32_t account_id, uint64_t call_generation, const char *registrar,
                       const char *username, const char *password, const char *destination, int transport, int media_security, const char *auth_username, const char *outbound_proxy) {
    struct engine *e = handle;
    struct request *r;
    int error;
    unsigned pending = 0;
    if (operation == OP_STOP) {
        LOCK(&e->lock); e->stop_requested = true; UNLOCK(&e->lock);
        return 0;
    }
    r = calloc(1, sizeof(*r));
    if (!r) return ENOMEM;
    r->transport = transport; r->media_security = media_security;
    r->auth_username = copy_string(auth_username); r->outbound_proxy = copy_string(outbound_proxy);
    r->operation = operation;
    r->account_id = account_id;
    r->call_generation = call_generation;
    r->registrar = copy_string(registrar); r->username = copy_string(username);
    r->password = copy_string(password); r->destination = copy_string(destination);
    if (!r->registrar || !r->username || !r->password || !r->destination || !r->auth_username || !r->outbound_proxy) { free_request(r); return ENOMEM; }
    LOCK(&e->lock);
    for (struct request *p = e->pending; p; p = p->next) ++pending;
    if (!e->running || e->stop_requested) error = ENOTCONN;
    else if (pending >= 16) error = EBUSY;
    else {
        r->next = e->pending; e->pending = r;
        error = mqueue_push(e->queue, operation, r);
        if (error) remove_pending(e, r);
    }
    UNLOCK(&e->lock);
    if (error) free_request(r);
    return error;
}

void sip_engine_destroy(void *handle) {
    struct engine *e = handle;
    if (!e) return;
    LOCK_CLOSE(&e->lock);
    free(e);
}

#ifdef SIP_NATIVE_TEST_HOOKS
int sip_engine_set_test_dns(void *handle, const char *ip, uint16_t port) {
    struct engine *e = handle;
    struct sa server;
    if (!e || !ip || !port || e->running || strlen(ip)>=sizeof(e->test_dns_ip)
        || sa_set_str(&server,ip,port) || !sa_is_loopback(&server)) return EINVAL;
    str_ncpy(e->test_dns_ip,ip,sizeof(e->test_dns_ip));e->test_dns_port=port;return 0;
}
int sip_engine_set_test_ca(void *handle, const char *path) {
    struct engine *e = handle;
    if (!e || !path || !path[0] || strlen(path) >= sizeof(e->test_ca) || e->running) return EINVAL;
    str_ncpy(e->test_ca, path, sizeof(e->test_ca)); return 0;
}
#endif
