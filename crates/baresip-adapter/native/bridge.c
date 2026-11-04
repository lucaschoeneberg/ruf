/* Private ABI owned by this project, compiled against the pinned upstream headers. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <re.h>
#include <rem.h>
#include <baresip.h>

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

typedef void (*event_callback)(int kind, int code, uint64_t call_generation, void *arg);
enum { EV_READY=1, EV_REGISTERING, EV_REGISTERED, EV_REG_FAILED,
       EV_CALLING, EV_RINGING, EV_INCOMING, EV_ACTIVE, EV_CLOSED, EV_ERROR, EV_STOPPED, EV_STALE };
enum { OP_ACCOUNT=1, OP_DIAL, OP_ANSWER, OP_HANGUP, OP_STOP };

struct request {
    struct request *next;
    int operation;
    uint64_t call_generation;
    char *registrar, *username, *password, *destination;
};
struct engine {
    struct mqueue *queue;
    queue_lock lock;
    bool running;
    bool stop_requested;
    struct tmr shutdown_timer;
    struct request *pending;
    struct ua *ua;
    struct call *call;
    uint64_t call_generation;
    char registrar[256];
    char recording[128];
    int audio_mode;
    event_callback callback;
    void *callback_arg;
};

static void emit(struct engine *e, int kind, int code) { e->callback(kind, code, e->call_generation, e->callback_arg); }
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
    free(r->registrar); free(r->username); free(r->password); free(r->destination); free(r);
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
    if (stop) { ua_stop_all(true); re_cancel(); }
    else tmr_start(&e->shutdown_timer, 50, shutdown_tick, e);
}

static void event_handler(enum bevent_ev kind, struct bevent *event, void *arg) {
    struct engine *e = arg;
    struct call *call = bevent_get_call(event);
    struct ua *ua = bevent_get_ua(event);
    if (ua && ua != e->ua) return;
    switch (kind) {
    case BEVENT_REGISTERING: emit(e, EV_REGISTERING, 0); break;
    case BEVENT_REGISTER_OK: emit(e, EV_REGISTERED, 0); break;
    case BEVENT_REGISTER_FAIL: emit(e, EV_REG_FAILED, 0); break;
    case BEVENT_CALL_INCOMING:
        if (e->call && e->call != call) { ua_hangup(e->ua, call, 486, "Busy Here"); break; }
        if (e->call != call) ++e->call_generation;
        e->call = call; emit(e, EV_INCOMING, 0); break;
    case BEVENT_CALL_OUTGOING: ++e->call_generation; e->call = call; emit(e, EV_CALLING, 0); break;
    case BEVENT_CALL_RINGING: emit(e, EV_RINGING, 0); break;
    case BEVENT_CALL_ESTABLISHED: emit(e, EV_ACTIVE, 0); break;
    case BEVENT_CALL_CLOSED:
        if (e->call == call) { e->call = NULL; emit(e, EV_CLOSED, call_scode(call)); }
        break;
    default: break;
    }
}

static void queue_handler(int id, void *data, void *arg) {
    struct engine *e = arg;
    struct request *r = data;
    int error = 0;
    char uri[512];
    (void)id;
    LOCK(&e->lock); remove_pending(e, r); UNLOCK(&e->lock);
    if ((r->operation == OP_ANSWER || r->operation == OP_HANGUP)
        && r->call_generation != e->call_generation) {
        emit(e, EV_STALE, 0);
        free_request(r);
        return;
    }
    switch (r->operation) {
    case OP_ACCOUNT: {
        struct ua *previous = e->ua;
        if (e->call) {
            emit(e, e->ua && ua_isregistered(e->ua) ? EV_REGISTERED : EV_REG_FAILED, 0);
            error = EBUSY; break;
        }
        e->ua = NULL;
        mem_deref(previous);
        (void)snprintf(uri, sizeof(uri), "<sip:%s@%s>;regint=60;audio_codecs=PCMU/8000/1,PCMA/8000/1", r->username, r->registrar);
        error = ua_alloc(&e->ua, uri);
        if (!error) error = account_set_auth_user(ua_account(e->ua), r->username);
        if (!error) error = account_set_auth_pass(ua_account(e->ua), r->password);
        if (!error) error = ua_register(e->ua);
        if (!error) str_ncpy(e->registrar, r->registrar, sizeof(e->registrar));
        if (error) emit(e, EV_REG_FAILED, error);
        break;
    }
    case OP_DIAL:
        if (!e->ua || !ua_isregistered(e->ua)) { error = ENOTCONN; break; }
        if (e->call) { error = EBUSY; break; }
        (void)snprintf(uri, sizeof(uri), "sip:%s@%s", r->destination, e->registrar);
        error = ua_connect(e->ua, &e->call, NULL, uri, VIDMODE_OFF);
        if (error) { e->call = NULL; emit(e, EV_CLOSED, error); }
        break;
    case OP_ANSWER:
        error = e->call ? ua_answer(e->ua, e->call, VIDMODE_OFF) : EINVAL;
        break;
    case OP_HANGUP:
        if (e->call) ua_hangup(e->ua, e->call, 0, NULL);
        else error = EINVAL;
        break;
    case OP_STOP:
        ua_stop_all(true);
        re_cancel();
        break;
    default: error = EINVAL;
    }
    if (error && r->operation == OP_DIAL && !e->call) emit(e, EV_CLOSED, error);
    if (error) emit(e, EV_ERROR, error);
    free_request(r);
}

void *sip_engine_create(event_callback callback, void *arg, int audio_mode, const char *recording) {
    struct engine *e = calloc(1, sizeof(*e));
    if (!e || !callback) { free(e); return NULL; }
    LOCK_INIT(&e->lock);
    e->callback = callback; e->callback_arg = arg; e->audio_mode = audio_mode;
    str_ncpy(e->recording, recording ? recording : "", sizeof(e->recording));
    return e;
}

int sip_engine_run(void *handle) {
    struct engine *e = handle;
    const char config[] = "sip_listen 0.0.0.0:0\nsip_transports udp\ncall_max_calls 1\nrtp_ports 40000-40100\n";
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
        str_ncpy(audio->src_mod, "audiounit", sizeof(audio->src_mod));
        str_ncpy(audio->play_mod, "audiounit", sizeof(audio->play_mod));
#elif defined(WIN32)
        str_ncpy(audio->src_mod, "wasapi", sizeof(audio->src_mod));
        str_ncpy(audio->play_mod, "wasapi", sizeof(audio->play_mod));
#else
        error = ENOTSUP;
#endif
    }
    if (!error) error = baresip_init(conf_config());
    /* Upstream auto-enumeration excludes IPv4 loopback. Local PBX calls still
       need a matching media address; add it through the existing network API. */
    if (!error) {
        struct sa loopback;
        error = sa_set_str(&loopback, "127.0.0.1", 0);
        if (!error) error = net_add_address(baresip_network(), &loopback);
    }
    if (!error) error = ua_init("SIP native milestone", true, false, false);
    if (!error) error = bevent_register(event_handler, e);
    if (!error) error = module_load("", "g711");
    if (!error) error = module_load("", "auconv");
    if (!error) error = module_load("", "auresamp");
    if (!error) error = module_load("", "ausine");
    if (!error) error = module_load("", "aufile");
#ifdef __APPLE__
    if (!error && !e->audio_mode) error = module_load("", "audiounit");
#elif defined(WIN32)
    if (!error && !e->audio_mode) error = module_load("", "wasapi");
#endif
    LOCK(&e->lock);
    if (!error) error = mqueue_alloc(&e->queue, queue_handler, e);
    e->running = !error;
    UNLOCK(&e->lock);
    emit(e, EV_READY, error);
    if (!error) {
        tmr_start(&e->shutdown_timer, 50, shutdown_tick, e);
        error = re_main(NULL);
    }
    tmr_cancel(&e->shutdown_timer);
    LOCK(&e->lock);
    e->running = false;
    e->queue = mem_deref(e->queue);
    while (e->pending) { struct request *r = e->pending; e->pending = r->next; free_request(r); }
    UNLOCK(&e->lock);
    if (initialized) {
        bevent_unregister(event_handler);
        e->call = NULL; e->ua = NULL;
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

int sip_engine_request(void *handle, int operation, uint64_t call_generation, const char *registrar,
                       const char *username, const char *password, const char *destination) {
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
    r->operation = operation;
    r->call_generation = call_generation;
    r->registrar = copy_string(registrar); r->username = copy_string(username);
    r->password = copy_string(password); r->destination = copy_string(destination);
    if (!r->registrar || !r->username || !r->password || !r->destination) { free_request(r); return ENOMEM; }
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
