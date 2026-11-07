/* Real macOS resolver lifetime regression; no SIP accounts or DNS queries.
 * This exceeds libnotify's per-process registration limit if resolver
 * state is only closed instead of destroyed. */
#include <assert.h>
#include <stdio.h>
#include <re.h>

enum { REFRESHES = 65536 };

int main(void) {
    for (unsigned i = 0; i < REFRESHES; ++i) {
        struct sa servers[8];
        uint32_t count = RE_ARRAY_SIZE(servers);
        assert(!dns_srv_get(NULL, 0, servers, &count));
        assert(count > 0 && count <= RE_ARRAY_SIZE(servers));
    }
    puts("DNS_REFRESH_OK: 65536 native DNS refreshes completed");
    return 0;
}
