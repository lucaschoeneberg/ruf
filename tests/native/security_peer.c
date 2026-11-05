/* Exercise the real transport authority predicate with only TLS verification stubbed. */
#include <assert.h>
#include <stdio.h>
#include <stdbool.h>
#define tls_peer_verified_server test_tls_verified
#define tcp_send test_tcp_send
#include PEER_SOURCE
bool test_tls_verified(const struct tls_conn *tls) { return tls && *(const bool *)tls; }
static unsigned sends;
int test_tcp_send(struct tcp_conn *tcp, struct mbuf *packet) { (void)tcp;(void)packet;++sends;return 0; }
int main(void) {
 bool verified=true,unverified=false;
 struct sip_conn a={0},b={0},same_host={0},fresh={0};struct sip_msg msg={0};
 struct sip_conn *peers[]={&a,&b,&same_host,&fresh};
 for (size_t i=0;i<4;i++) { peers[i]->tp=SIP_TRANSP_TLS;peers[i]->established=true;peers[i]->tc=(void *)1;peers[i]->sc=(void *)&verified; }
 a.tls_verify_host="a.test";b.tls_verify_host="b.test";same_host.tls_verify_host="a.test";fresh.sc=(void *)&unverified;
 msg.tp=SIP_TRANSP_TLS;msg.sock=&fresh;assert(!sip_tls_peer_matches(&msg,&a,"a.test"));
 msg.sock=&b;assert(!sip_tls_peer_matches(&msg,&a,"a.test"));
 msg.sock=&same_host;assert(sip_tls_peer_matches(&msg,&a,"a.test"));
 msg.sock=&a;assert(sip_tls_peer_matches(&msg,&a,"a.test"));
 msg.sock=&fresh;assert(sip_tls_peer_matches(&msg,&fresh,NULL));assert(!sip_tls_peer_matches(&msg,&a,NULL));
 fresh.established=false;assert(!sip_tls_peer_matches(&msg,&fresh,NULL));
 struct sip sip={0};struct sa destination;struct mbuf *packet=mbuf_alloc(32);
 assert(!sa_set_str(&destination,"127.0.0.1",5061));fresh.paddr=destination;fresh.sip=&sip;fresh.established=true;
 assert(!sip_transp_send(NULL,&sip,&fresh,SIP_TRANSP_TLS,&destination,NULL,packet,NULL,NULL,NULL));assert(sends==1);
 sa_set_port(&destination,5062);
 assert(sip_transp_send(NULL,&sip,&fresh,SIP_TRANSP_TLS,&destination,NULL,packet,NULL,NULL,NULL));assert(sends==1);
 destination=fresh.paddr;fresh.established=false;
 assert(sip_transp_send(NULL,&sip,&fresh,SIP_TRANSP_TLS,&destination,NULL,packet,NULL,NULL,NULL));assert(sends==1);
 mem_deref(packet);
 puts("actual TLS authority: original unverified socket or matching verified host only: PASS");return 0;
}
