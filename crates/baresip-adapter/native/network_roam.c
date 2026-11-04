/* Idle-only network recovery using the pinned netroam module's native APIs. */
#include <string.h>
#include <re.h>
#include <baresip.h>
#include "network_roam.h"

enum { POLL_MS = 5000, MAX_RETRY_MS = 60000 };

static struct {
    bool pending;
    unsigned failures;
    uint64_t next_attempt;
} recovery;

struct scan {
    struct network *net;
    struct sa obsolete;
    bool changed;
    int error;
};

static bool address_matches(const char *name, const struct sa *address, void *arg) {
    (void)name;
    return sa_cmp(address, arg, SA_ADDR);
}

static bool add_missing(const char *name, const struct sa *address, void *arg) {
    struct scan *scan = arg;
    /* Use upstream's AF, interface and link-local policy; utun interfaces are
     * eligible, while bridge-managed loopback is left untouched. */
    if (!net_ifaddr_filter(scan->net, name, address) ||
            net_laddr_apply(scan->net, address_matches, (void *)address))
        return false;
    scan->error = net_add_address_ifname(scan->net, address, name);
    if (scan->error) return true;
    scan->changed = true;
    return false;
}

static bool find_obsolete(const char *name, const struct sa *address, void *arg) {
    struct scan *scan = arg;
    char current_name[256];
    (void)name;
    int error = net_if_getname(current_name, sizeof(current_name), sa_af(address), address);
    if (error == ENODEV) {
        scan->obsolete = *address;
        return true;
    }
    /* An inventory failure is not evidence that an address disappeared. */
    scan->error = error;
    return error != 0;
}

static int retry_later(uint64_t now, int error) {
    unsigned delay = POLL_MS << recovery.failures;
    if (delay > MAX_RETRY_MS) delay = MAX_RETRY_MS;
    if (recovery.failures < 4) ++recovery.failures;
    recovery.next_attempt = now + delay;
    return error;
}

void sip_network_reset(void) {
    memset(&recovery, 0, sizeof(recovery));
}

int sip_network_poll(bool call_active) {
    /* No network mutation is allowed while the bridge holds any call slot. */
    if (call_active) return 0;
    uint64_t now = tmr_jiffies();
    if (now < recovery.next_attempt) return 0;
    struct network *net = baresip_network();
    if (!net) return ENODEV;

    net_dns_refresh(net);
    struct scan scan = { .net = net };
    int error = net_if_apply(add_missing, &scan);
    if (!error) error = scan.error;
    if (scan.changed) recovery.pending = true;
    if (error) return retry_later(now, error);

    /* Do not remove inside net_laddr_apply: its iterator borrows the entry. */
    while (net_laddr_apply(net, find_obsolete, &scan)) {
        if (scan.error) return retry_later(now, scan.error);
        error = net_rm_address(net, &scan.obsolete);
        if (error) return retry_later(now, error);
        scan.changed = recovery.pending = true;
    }

    if (scan.changed) {
        /* Require one quiet poll before rebinding all transports. */
        recovery.failures = 0;
        recovery.next_attempt = now + POLL_MS;
        return 0;
    }
    if (recovery.pending) {
        error = uag_reset_transp(true, true);
        if (error) return retry_later(now, error);
        recovery.pending = false;
    }
    recovery.failures = 0;
    recovery.next_attempt = 0;
    return 0;
}
