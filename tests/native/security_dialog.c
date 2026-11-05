/* Native TLS floor regression: plaintext messages must not match or advance CSeq. */
#include <assert.h>
#include <stdio.h>
#include DIALOG_SOURCE
#include <re_main.h>
int main(void) {
 struct sip_dialog *dialog=NULL;struct sip_msg *msg=NULL;struct mbuf *packet=NULL;
 const char *text="INVITE sip:alice@localhost SIP/2.0\r\nVia: SIP/2.0/TLS localhost;branch=z9hG4bK-test\r\nFrom: <sip:bob@localhost>;tag=peer\r\nTo: <sip:alice@localhost>\r\nCall-ID: secure-dialog-test\r\nCSeq: 1 INVITE\r\nContact: <sip:bob@127.0.0.1:5061>\r\nContent-Length: 0\r\n\r\n";
 assert(!libre_init());packet=mbuf_alloc(strlen(text));assert(packet);assert(!mbuf_write_str(packet,text));packet->pos=0;
 assert(!sip_msg_decode(&msg,packet));msg->tp=SIP_TRANSP_TLS;assert(!sip_dialog_accept(&dialog,msg));
 msg->to.tag=(struct pl){dialog->ltag,strlen(dialog->ltag)};assert(sip_dialog_cmp(dialog,msg));
 msg->tp=SIP_TRANSP_UDP;msg->cseq.num=999;
 assert(!sip_dialog_cmp(dialog,msg));assert(!sip_dialog_rseq_valid(dialog,msg));
 msg->tp=SIP_TRANSP_TLS;msg->cseq.num=2;assert(sip_dialog_cmp(dialog,msg));assert(sip_dialog_rseq_valid(dialog,msg));
 mem_deref(msg);mem_deref(packet);mem_deref(dialog);libre_close();puts("TLS dialog plaintext match/CSeq rejection: PASS");return 0;
}
