/* Plaintext final/provisional responses cannot mutate a TLS client transaction. */
#include <assert.h>
#include <stdio.h>
#include CTRANS_SOURCE
int main(void) {
 struct sip_ctrans transaction={0};struct sip_msg message={0};
 transaction.tp=SIP_TRANSP_TLS;transaction.branch="z9hG4bK-secure";transaction.met="INVITE";transaction.he.data=&transaction;
 pl_set_str(&message.via.branch,transaction.branch);pl_set_str(&message.cseq.met,transaction.met);
 message.tp=SIP_TRANSP_UDP;assert(!cmp_handler(&transaction.he,&message));
 message.tp=SIP_TRANSP_TCP;assert(!cmp_handler(&transaction.he,&message));
 message.tp=SIP_TRANSP_TLS;assert(cmp_handler(&transaction.he,&message));
 puts("TLS client transaction rejects plaintext responses: PASS");return 0;
}
