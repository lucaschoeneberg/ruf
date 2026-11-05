/* Plaintext final/provisional responses cannot mutate a TLS client transaction. */
#include <assert.h>
#include <stdio.h>
#include "peer_authority_fixture.h"
#include CTRANS_SOURCE
#include "peer_authority_fixture_impl.h"
int main(void) {
 struct sip_ctrans transaction={0};struct sip_msg message={0};
 struct test_peer a={"a.test",true},b={"b.test",true},fresh={NULL,true};
 transaction.host="a.test";
#ifndef SECURITY_UPSTREAM_RED
 transaction.tls_peer_sock=&a;
#endif
 transaction.tp=SIP_TRANSP_TLS;transaction.branch="z9hG4bK-secure";transaction.met="INVITE";transaction.he.data=&transaction;
 pl_set_str(&message.via.branch,transaction.branch);pl_set_str(&message.cseq.met,transaction.met);
 message.tp=SIP_TRANSP_UDP;assert(!cmp_handler(&transaction.he,&message));
 message.tp=SIP_TRANSP_TCP;assert(!cmp_handler(&transaction.he,&message));
 message.tp=SIP_TRANSP_TLS;message.sock=&fresh;assert(!cmp_handler(&transaction.he,&message));message.sock=&b;assert(!cmp_handler(&transaction.he,&message));message.sock=&a;assert(cmp_handler(&transaction.he,&message));
#ifndef SECURITY_UPSTREAM_RED
 transaction.host=NULL; /* direct original-channel dialog request */
 message.sock=&fresh;assert(!cmp_handler(&transaction.he,&message));message.sock=&b;assert(!cmp_handler(&transaction.he,&message));message.sock=&a;assert(cmp_handler(&transaction.he,&message));
#endif
 puts("TLS client transaction rejects plaintext responses: PASS");return 0;
}
