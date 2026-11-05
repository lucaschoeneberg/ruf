/* Explicit dialog transport must bypass insecure NAPTR/SRV transport fallback. */
#include <assert.h>
#include <stdio.h>
#ifndef SECURITY_UPSTREAM_RED
#define sip_ctrans_request test_bound_ctrans
#define sip_send_conn test_bound_send
#endif
#include REQUEST_SOURCE
#include <re_main.h>
#ifndef SECURITY_UPSTREAM_RED
static void *sent_socket;
static unsigned sends;
int test_bound_send(struct sip *sip, void *socket, enum sip_transp tp, const struct sa *dst, char *host, struct mbuf *mb, sip_conn_h *connh, void *arg) {
 (void)sip;(void)tp;(void)dst;(void)mb;(void)connh;(void)arg;assert(!host);sent_socket=socket;++sends;return 0;
}
int test_bound_ctrans(struct sip_ctrans **ctp, struct sip *sip, void *socket, enum sip_transp tp, const struct sa *dst, char *met, char *branch, char *host, struct mbuf *mb, sip_conn_h *connh, sip_resp_h *resph, void *arg) {
 (void)ctp;(void)met;(void)branch;(void)resph;return test_bound_send(sip,socket,tp,dst,host,mb,connh,arg);
}
#endif
int main(void) {
 struct sip sip={0};struct sip_request *req=NULL;struct uri route;struct pl text;struct mbuf *body=mbuf_alloc(64);
 assert(!libre_init());pl_set_str(&text,"sip:peer@localhost");assert(!uri_decode(&route,&text));
 assert(!sip_request_alloc(&req,&sip,true,"INVITE",-1,"sip:peer@localhost",-1,&route,SIP_TRANSP_TLS,body,0,NULL,NULL,NULL));
 assert(req->tp==SIP_TRANSP_TLS);assert(req->tp_selected);
#ifndef SECURITY_UPSTREAM_RED
 req->tls_required=true;req->tls_peer_sock=mem_zalloc(1,NULL);assert(!sa_set_str(&req->tls_peer_addr,"127.0.0.1",5061));
 assert(!request(req,SIP_TRANSP_TLS,&req->tls_peer_addr));assert(sent_socket==req->tls_peer_sock && sends==1);
 req->stateful=false;assert(!request(req,SIP_TRANSP_TLS,&req->tls_peer_addr));assert(sent_socket==req->tls_peer_sock && sends==2);
 struct sa changed=req->tls_peer_addr;sa_set_port(&changed,5062);
 assert(request(req,SIP_TRANSP_TLS,&changed));assert(sends==2);
 assert(request(req,SIP_TRANSP_UDP,&req->tls_peer_addr));assert(sends==2);
#endif
 mem_deref(req);mem_deref(body);libre_close();puts("explicit TLS transport selection: PASS");return 0;
}
