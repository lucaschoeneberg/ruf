/* Matching plaintext ACK/CANCEL cannot terminate an incoming TLS transaction. */
#include <assert.h>
#include <stdio.h>
#include "peer_authority_fixture.h"
#include STRANS_SOURCE
#include "peer_authority_fixture_impl.h"
int main(void) {
 struct sip_strans transaction={0};struct sip_msg original={0},message={0};
 struct test_peer a={"a.test",true},b={"b.test",true},fresh={NULL,true};
 original.sock=&a;
#ifndef SECURITY_UPSTREAM_RED
 transaction.tls_peer_host="a.test";
#endif
 original.tp=SIP_TRANSP_TLS;pl_set_str(&original.via.branch,"z9hG4bK-secure");pl_set_str(&original.via.sentby,"localhost");pl_set_str(&original.cseq.met,"INVITE");
 message=original;message.tp=SIP_TRANSP_UDP;transaction.msg=&original;transaction.he.data=&transaction;
 assert(!cmp_handler(&transaction.he,&message));assert(!cmp_ack_handler(&transaction.he,&message));assert(!cmp_cancel_handler(&transaction.he,&message));
 message.tp=SIP_TRANSP_TLS;message.sock=&fresh;assert(!cmp_handler(&transaction.he,&message));assert(!cmp_ack_handler(&transaction.he,&message));assert(!cmp_cancel_handler(&transaction.he,&message));message.sock=&b;assert(!cmp_handler(&transaction.he,&message));assert(!cmp_ack_handler(&transaction.he,&message));assert(!cmp_cancel_handler(&transaction.he,&message));message.sock=&a;assert(cmp_handler(&transaction.he,&message));assert(cmp_ack_handler(&transaction.he,&message));assert(cmp_cancel_handler(&transaction.he,&message));
#ifndef SECURITY_UPSTREAM_RED
 transaction.tls_peer_host=NULL;original.sock=&fresh;message.sock=&fresh;assert(cmp_handler(&transaction.he,&message));assert(cmp_ack_handler(&transaction.he,&message));assert(cmp_cancel_handler(&transaction.he,&message));message.sock=&b;assert(!cmp_handler(&transaction.he,&message));assert(!cmp_ack_handler(&transaction.he,&message));assert(!cmp_cancel_handler(&transaction.he,&message));
#endif
 puts("TLS server transaction rejects plaintext ACK/CANCEL: PASS");return 0;
}
