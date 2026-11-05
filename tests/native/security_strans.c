/* Matching plaintext ACK/CANCEL cannot terminate an incoming TLS transaction. */
#include <assert.h>
#include <stdio.h>
#include STRANS_SOURCE
int main(void) {
 struct sip_strans transaction={0};struct sip_msg original={0},message={0};
 original.tp=SIP_TRANSP_TLS;pl_set_str(&original.via.branch,"z9hG4bK-secure");pl_set_str(&original.via.sentby,"localhost");pl_set_str(&original.cseq.met,"INVITE");
 message=original;message.tp=SIP_TRANSP_UDP;transaction.msg=&original;transaction.he.data=&transaction;
 assert(!cmp_handler(&transaction.he,&message));assert(!cmp_ack_handler(&transaction.he,&message));assert(!cmp_cancel_handler(&transaction.he,&message));
 message.tp=SIP_TRANSP_TLS;assert(cmp_handler(&transaction.he,&message));assert(cmp_ack_handler(&transaction.he,&message));assert(cmp_cancel_handler(&transaction.he,&message));
 puts("TLS server transaction rejects plaintext ACK/CANCEL: PASS");return 0;
}
