/* Explicit dialog transport must bypass insecure NAPTR/SRV transport fallback. */
#include <assert.h>
#include <stdio.h>
#include REQUEST_SOURCE
#include <re_main.h>
int main(void) {
 struct sip sip={0};struct sip_request *request=NULL;struct uri route;struct pl text;struct mbuf *body=mbuf_alloc(64);
 assert(!libre_init());pl_set_str(&text,"sip:peer@localhost");assert(!uri_decode(&route,&text));
 assert(!sip_request_alloc(&request,&sip,true,"INVITE",-1,"sip:peer@localhost",-1,&route,SIP_TRANSP_TLS,body,0,NULL,NULL,NULL));
 assert(request->tp==SIP_TRANSP_TLS);assert(request->tp_selected);
 mem_deref(request);mem_deref(body);libre_close();puts("explicit TLS transport selection: PASS");return 0;
}
