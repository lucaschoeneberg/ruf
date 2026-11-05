#ifndef SIP_CALL_NOTIFICATIONS_H
#define SIP_CALL_NOTIFICATIONS_H

/* Process-lifetime callback; no borrowed context or per-call native callback map.
 * action: 1 accept, 2 reject, 3 open. token is borrowed NUL-terminated ASCII,
 * 1..128 bytes, valid only during this synchronous callback. No caller data. */
typedef void (*sip_call_notification_callback)(int action, const char *token);

/* Initializes the delegate/category without prompting. Requires a matching .app
 * bundle identifier; safe on the UI thread. callback must remain valid forever.
 * Returns zero on success; nonzero on unavailable/invalid native context. */
int sip_call_notifications_init(const char *app_id, sip_call_notification_callback callback);

/* All remaining functions are synchronous and must run on a blocking worker.
 * Permission: 0 undecided, 1 denied, 2 authorized, 3 unsupported launch context.
 * request must be zero unless the person explicitly requested foreground access.
 * An ordinary denial is returned as permission=1, not as an execution failure. */
/* Service queries time out after five seconds; an explicit authorization decision
 * has a separate three-minute deadline. A timeout is an execution failure. */
int sip_call_notifications_permission(int request, int *permission);

/* Silent immediate delivery with fixed accept/reject actions. Never requests
 * authorization. Waits for OS acceptance, cancellation by dismiss or timeout.
 * Nonzero reports native execution failure, a five-second service timeout or
 * missing authorization. A late OS callback remains safe after a timeout. */
int sip_call_notifications_show(const char *token, const char *title, const char *body);

/* Invalidates in-flight delivery and removes pending and delivered requests for
 * this exact token. Unknown tokens are harmless. No authorization prompt. */
int sip_call_notifications_dismiss(const char *token);

#endif
