/* Test connections for exercising production dispatch guard call sites. */
#include <stdbool.h>
#include <string.h>
struct test_peer { const char *verified_host; bool live; };
#define sip_tls_peer_matches test_peer_matches
#define sip_msg_tls_peer_verified_host test_verified_host
