/* Standalone state-machine tests with real SDK address types/functions.
 * Only OS enumeration, baresip network storage, DNS and SIP effects are faked.
 * Compile this file and network_roam.c against the installed libre SDK. */
/* Assertions are the test oracle, including in Release builds. */
#undef NDEBUG
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <re.h>
#include <baresip.h>
#include "../network_roam.h"

struct address {
    const char *name;
    struct sa sa;
};
struct network { struct address addresses[16]; size_t count; };
static struct network cached;
static struct address interfaces[16];
static size_t interface_count;
static unsigned dns_refreshes, additions, removals, resets;
static int enumerate_error, add_error, reset_error;
static uint64_t now;

uint64_t tmr_jiffies(void) { return now; }
struct network *baresip_network(void) { return &cached; }
void net_dns_refresh(struct network *net) {
    assert(net == &cached);
    ++dns_refreshes;
}
int net_if_apply(net_ifaddr_h *handler, void *arg) {
    if (enumerate_error) return enumerate_error;
    for (size_t i = 0; i < interface_count; ++i)
        if (handler(interfaces[i].name, &interfaces[i].sa, arg)) break;
    return 0;
}
bool net_ifaddr_filter(const struct network *net, const char *name, const struct sa *sa) {
    assert(net == &cached);
    (void)name;
    return !sa_is_loopback(sa) && !sa_is_linklocal(sa);
}
bool net_laddr_apply(const struct network *net, net_ifaddr_h *handler, void *arg) {
    for (size_t i = 0; i < net->count; ++i)
        if (handler(net->addresses[i].name, &net->addresses[i].sa, arg)) return true;
    return false;
}
int net_add_address_ifname(struct network *net, const struct sa *sa, const char *name) {
    assert(net == &cached);
    if (add_error) return add_error;
    assert(net->count < RE_ARRAY_SIZE(net->addresses));
    net->addresses[net->count++] = (struct address){ .name = name, .sa = *sa };
    ++additions;
    return 0;
}
int net_rm_address(struct network *net, const struct sa *sa) {
    assert(net == &cached);
    for (size_t i = 0; i < net->count; ++i) {
        if (!sa_cmp(&net->addresses[i].sa, sa, SA_ADDR)) continue;
        memmove(&net->addresses[i], &net->addresses[i + 1],
                (net->count - i - 1) * sizeof(net->addresses[0]));
        --net->count;
        ++removals;
        return 0;
    }
    return ENODEV;
}
int net_if_getname(char *name, size_t size, int af, const struct sa *sa) {
    if (enumerate_error) return enumerate_error;
    for (size_t i = 0; i < interface_count; ++i) {
        if (af != sa_af(&interfaces[i].sa) ||
                !sa_cmp(&interfaces[i].sa, sa, SA_ADDR)) continue;
        snprintf(name, size, "%s", interfaces[i].name);
        return 0;
    }
    return ENODEV;
}
int uag_reset_transp(bool reg, bool reinvite) {
    assert(reg && reinvite);
    ++resets;
    return reset_error;
}

static void add_interface(const char *name, const char *ip) {
    assert(interface_count < RE_ARRAY_SIZE(interfaces));
    interfaces[interface_count].name = name;
    assert(!sa_set_str(&interfaces[interface_count].sa, ip, 0));
    ++interface_count;
}
static void fixture(void) {
    sip_network_reset();
    memset(&cached, 0, sizeof(cached));
    interface_count = dns_refreshes = additions = removals = resets = 0;
    enumerate_error = add_error = reset_error = 0;
    now = 1000;
    add_interface("lo0", "127.0.0.1");
    add_interface("en0", "192.0.2.1");
    /* The bridge explicitly retains loopback for the local PBX. */
    cached.addresses[cached.count++] = interfaces[0];
    cached.addresses[cached.count++] = interfaces[1];
}
static int tick(bool active) {
    now += 5000;
    return sip_network_poll(active);
}

/* Break caught: ignoring newly appeared utun interfaces or resetting before
 * all added/removed addresses have settled. */
static void late_tunnel_and_disconnect(void) {
    fixture();
    assert(!tick(false));
    assert(dns_refreshes == 1 && resets == 0);
    add_interface("utun8", "100.64.0.2");
    add_interface("utun8", "fd7a:115c:a1e0::2");
    assert(!tick(false));
    assert(cached.count == 4 && additions == 2 && resets == 0);
    assert(!tick(false));
    assert(resets == 1);
    assert(!tick(false));
    assert(resets == 1 && additions == 2);
    interface_count = 2;
    assert(!tick(false));
    assert(cached.count == 2 && removals == 2 && resets == 1);
    assert(!tick(false));
    assert(resets == 2 && sa_is_loopback(&cached.addresses[0].sa));
}

/* Break caught: mutating DNS/addresses or resetting a pending transport while
 * any incoming, answering, outgoing or established call occupies the slot. */
static void active_call_defers_every_mutation(void) {
    fixture();
    add_interface("utun8", "100.64.0.2");
    assert(!tick(true));
    assert(!dns_refreshes && !additions && !removals && !resets);
    assert(!tick(false));
    assert(additions == 1 && dns_refreshes == 1);
    assert(!tick(true));
    assert(!resets && dns_refreshes == 1);
    interface_count = 2;
    assert(!tick(true));
    assert(!removals && cached.count == 3 && dns_refreshes == 1);
    assert(!tick(false));
    assert(removals == 1 && !resets);
    assert(!tick(false));
    assert(resets == 1);
}

/* Break caught: resetting during successive address changes. */
static void changes_debounce_transport_reset(void) {
    fixture();
    add_interface("utun8", "100.64.0.2");
    assert(!tick(false));
    add_interface("utun8", "fd7a:115c:a1e0::2");
    assert(!tick(false));
    assert(additions == 2 && !resets);
    assert(!tick(false));
    assert(resets == 1);
}

/* Break caught: tight-loop reset retries, abandoning retries, or an overflow
 * that permits retry delays above sixty seconds. */
static void reset_retries_have_bounded_backoff(void) {
    fixture();
    add_interface("utun8", "100.64.0.2");
    assert(!tick(false));
    reset_error = EIO;
    assert(tick(false) == EIO);
    assert(resets == 1);
    now += 5000;
    assert(sip_network_poll(false) == EIO && resets == 2);
    now += 5000;
    assert(!sip_network_poll(false) && resets == 2);
    now += 5000;
    assert(sip_network_poll(false) == EIO && resets == 3);
    now += 20000;
    assert(sip_network_poll(false) == EIO && resets == 4);
    now += 40000;
    assert(sip_network_poll(false) == EIO && resets == 5);
    now += 55000;
    assert(!sip_network_poll(false) && resets == 5);
    now += 5000;
    assert(sip_network_poll(false) == EIO && resets == 6);
    /* A wake after the capped delay must immediately retry. */
    now += 120000;
    reset_error = 0;
    assert(!sip_network_poll(false) && resets == 7);
    assert(!tick(false) && resets == 7);
}

/* Break caught: swallowing an address update error or resetting before a
 * failed inventory query can be completed. */
static void inventory_failures_retry_without_reset(void) {
    fixture();
    add_interface("utun8", "100.64.0.2");
    add_error = ENOMEM;
    assert(tick(false) == ENOMEM);
    assert(cached.count == 2 && !resets);
    add_error = 0;
    assert(!tick(false));
    assert(cached.count == 3 && !resets);
    enumerate_error = EIO;
    assert(tick(false) == EIO);
    assert(cached.count == 3 && !removals && !resets);
    enumerate_error = 0;
    assert(!tick(false));
    assert(resets == 1);
}

/* Break caught: leaking pending recovery across engine lifetimes. */
static void cleanup_clears_pending_recovery(void) {
    fixture();
    add_interface("utun8", "100.64.0.2");
    assert(!tick(false));
    sip_network_reset();
    assert(!tick(false));
    assert(!resets);
}

int main(void) {
    late_tunnel_and_disconnect();
    active_call_defers_every_mutation();
    changes_debounce_transport_reset();
    reset_retries_have_bounded_backoff();
    inventory_failures_retry_without_reset();
    cleanup_clears_pending_recovery();
    puts("network_roam: 6 state-machine tests passed");
    return 0;
}
