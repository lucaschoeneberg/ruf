/* Private network recovery, called only on the singleton engine event loop. */
#ifndef SIP_NETWORK_ROAM_H
#define SIP_NETWORK_ROAM_H

#include <stdbool.h>

/* Clear deferred recovery/backoff before engine startup and after shutdown. */
void sip_network_reset(void);

/* Poll every five seconds. An occupied call slot (incoming through closed)
 * defers DNS/address mutations and transport resets. Idle address changes are
 * reconciled before a quiet poll resets transports and re-registers accounts.
 * Returns zero when unchanged/deferred/successful, otherwise an errno value.
 * Failed work retries with exponential backoff capped at sixty seconds. */
int sip_network_poll(bool call_active);

#endif
