#ifndef SIP_LOGIN_ITEM_H
#define SIP_LOGIN_ITEM_H

/* Return zero on success, nonzero on failure. Enabled is written as 0 or 1;
 * only SMAppServiceStatusEnabled is true. No native error text is exported. */
int sip_login_item_enabled(int *enabled);
/* Register/unregister only this signed main app. Approval-required is failure,
 * never enabled. Does not launch System Settings or alter legacy LaunchAgents. */
int sip_login_item_set_enabled(int enabled);
/* Read the current main-thread open-application Apple event. No side effects. */
int sip_login_item_launched_at_login(void);

#endif
