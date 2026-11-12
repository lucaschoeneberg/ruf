/* CPU-only comparison of native jitter-buffer profiles with a virtual RTP clock.
 * No network, microphone, speaker or real call is used. Not end-to-end latency.
 * Build/run instructions: native/README.md. */
#include <stdio.h>
#include <stdlib.h>
#include <re.h>
#include <baresip.h>
static uint64_t virtual_ms;
static uint64_t virtual_clock(const struct jbuf *unused) { (void)unused; return virtual_ms * 8; }
static int run(enum jbuf_type type, unsigned minimum, unsigned jitter_ms) {
 struct jbuf *jb = NULL;
 unsigned sent=0, played=0, dropped=0, first=0;
 void *payload = mem_zalloc(1, NULL);
 int error = jbuf_alloc(&jb, minimum, 200, 50);
 if(error || !payload) { mem_deref(payload); mem_deref(jb); return 1; }
 jbuf_set_srate(jb,8000); jbuf_set_type(jb,type); jbuf_set_next_play_h(jb,virtual_clock);
 for(virtual_ms=1000;virtual_ms<3500;++virtual_ms) {
  if(sent<100) {
   const unsigned variation=(sent%5)*jitter_ms/4;
   unsigned arrival=1000+sent*20+variation;
   if(virtual_ms>=arrival) {
    struct rtp_header hdr={0};hdr.ver=2;hdr.seq=(uint16_t)(sent+1);hdr.ts=sent*160;hdr.ts_arrive=virtual_ms*8;hdr.ssrc=1;
    if(jbuf_put(jb,&hdr,payload))++dropped;
    ++sent;
   }
  }
  for(;;) {
   struct rtp_header hdr={0};void *out=NULL;
   int result=jbuf_get(jb,&hdr,&out);
   if(out) { if(!played)first=(unsigned)virtual_ms-1000; ++played;mem_deref(out); }
   if(result!=EAGAIN)break;
  }
 }
 printf("{\"profile\":\"%s\",\"minimum_ms\":%u,\"input_jitter_ms\":%u,\"first_playout_ms\":%u,\"packets_sent\":%u,\"packets_played\":%u,\"put_errors\":%u}\n",type==JBUF_ADAPTIVE?"adaptive":"fixed",minimum,jitter_ms,first,sent,played,dropped);
 mem_deref(payload);mem_deref(jb);return played==100 && dropped==0 ? 0:1;
}
int main(void) {
 if(libre_init())return 1;
 int error=run(JBUF_FIXED,100,0)|run(JBUF_ADAPTIVE,40,0)|run(JBUF_FIXED,100,12)|run(JBUF_ADAPTIVE,40,12)|run(JBUF_FIXED,100,40)|run(JBUF_ADAPTIVE,40,40);
 libre_close();return error;
}
