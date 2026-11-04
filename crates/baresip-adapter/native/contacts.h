#ifndef SIP_CONTACTS_H
#define SIP_CONTACTS_H
#include <stdint.h>

/* All strings are borrowed UTF-8 values valid only during the synchronous callback.
 * An entry without phone numbers produces one callback with empty label/number.
 * Consecutive rows with the same identifier belong to one native unified contact. */
typedef void (*sip_contact_callback)(void *context, const char *identifier,
    const char *name, const char *organization, const char *label, const char *number);

/* Permission: 0 undecided, 1 restricted, 2 denied, 3 authorized.
 * Checks/requests authorization without enumerating contacts.
 * Returns zero on success (including denial), nonzero on authorization failure.
 * request_access must be zero unless the person explicitly requested access.
 * Both functions are synchronous and must run on a blocking worker. */
int sip_contacts_permission(int request_access, int *permission);

/* Uses the same authorization behavior; nonzero also reports read failures. */
int sip_contacts_read(int request_access, int *permission,
    sip_contact_callback callback, void *context);

/* Process-local revision only; never reads records or requests authorization.
 * The retained store/observer live until process exit. */
uint64_t sip_contacts_revision(void);

/* Copies validated UTF-8 text to the macOS pasteboard. May synchronously dispatch
 * to the main thread; call from a worker while the main event loop runs. */
int sip_copy_phone_number(const char *number);

#endif
