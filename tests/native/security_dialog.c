/* Native TLS floor regression: plaintext messages must not match or advance CSeq. */
#include <assert.h>
#include <stdio.h>
#include "peer_authority_fixture.h"
#include DIALOG_SOURCE
#include "peer_authority_fixture_impl.h"
#include <re_main.h>
int main(void) {
 struct test_peer *a=mem_zalloc(sizeof(*a),NULL), *b=mem_zalloc(sizeof(*b),NULL), *fresh=mem_zalloc(sizeof(*fresh),NULL);
 a->verified_host="a.test";a->live=true;b->verified_host="b.test";b->live=true;fresh->live=true;
 struct sip_dialog *dialog=NULL;struct sip_msg *msg=NULL;struct mbuf *packet=NULL;
 const char *text="INVITE sip:alice@localhost SIP/2.0\r\nVia: SIP/2.0/TLS localhost;branch=z9hG4bK-test\r\nFrom: <sip:bob@localhost>;tag=peer\r\nTo: <sip:alice@localhost>\r\nCall-ID: secure-dialog-test\r\nCSeq: 1 INVITE\r\nContact: <sip:bob@127.0.0.1:5061>\r\nContent-Length: 0\r\n\r\n";
 assert(!libre_init());packet=mbuf_alloc(strlen(text));assert(packet);assert(!mbuf_write_str(packet,text));packet->pos=0;
 assert(!sip_msg_decode(&msg,packet));msg->tp=SIP_TRANSP_TLS;msg->sock=mem_ref(a);assert(!sip_dialog_accept(&dialog,msg));
 msg->to.tag=(struct pl){dialog->ltag,strlen(dialog->ltag)};assert(sip_dialog_cmp(dialog,msg));
 msg->cseq.num=999;
 mem_deref(msg->sock);msg->sock=mem_ref(fresh);assert(!sip_dialog_cmp(dialog,msg));assert(!sip_dialog_rseq_valid(dialog,msg));assert(sip_dialog_update(dialog,msg));
 mem_deref(msg->sock);msg->sock=mem_ref(b);assert(!sip_dialog_cmp(dialog,msg));assert(!sip_dialog_rseq_valid(dialog,msg));assert(sip_dialog_update(dialog,msg));
 mem_deref(msg->sock);msg->sock=mem_ref(a);
 msg->tp=SIP_TRANSP_UDP;msg->cseq.num=999;
 assert(!sip_dialog_cmp(dialog,msg));assert(!sip_dialog_rseq_valid(dialog,msg));
 msg->tp=SIP_TRANSP_TLS;msg->cseq.num=2;assert(sip_dialog_cmp(dialog,msg));assert(sip_dialog_rseq_valid(dialog,msg));
 /* An initially unverified inbound TLS dialog stays confined to its original socket. */
 struct sip_dialog *inbound=NULL;mem_deref(msg->sock);msg->sock=mem_ref(fresh);msg->to.tag=pl_null;
 assert(!sip_dialog_accept(&inbound,msg));msg->to.tag=(struct pl){inbound->ltag,strlen(inbound->ltag)};assert(sip_dialog_cmp(inbound,msg));
#ifndef SECURITY_UPSTREAM_RED
 struct sa bound;assert(sip_dialog_tls_socket(inbound,&bound)==fresh);assert(!sip_dialog_tls_peer(inbound,&bound));
#endif
 mem_deref(msg->sock);msg->sock=mem_ref(b);assert(!sip_dialog_cmp(inbound,msg));
 mem_deref(inbound);mem_deref(msg);mem_deref(packet);mem_deref(dialog);mem_deref(a);mem_deref(b);mem_deref(fresh);libre_close();puts("TLS dialog plaintext match/CSeq rejection: PASS");return 0;
}
