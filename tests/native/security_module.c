/* Compile against original and patched module to demonstrate behavioral RED/GREEN. */
#include <assert.h>
#include <stdio.h>
#ifndef SRTP_SOURCE
#error SRTP_SOURCE must name the pinned module source
#endif
#include SRTP_SOURCE

int main(void) {
    struct sa local; struct sdp_session *sdp=NULL; struct sdp_media *media=NULL;
    struct menc_sess *session=NULL; struct menc_media *opaque=NULL;
    assert(!libre_init()); assert(!sa_set_str(&local,"127.0.0.1",0));
    assert(!sdp_session_alloc(&sdp,&local));
    assert(!sdp_media_add(&media,sdp,"audio",9000,"RTP/SAVP"));
    assert(!sdp_format_add(NULL,media,false,"0","PCMU",8000,1,NULL,NULL,NULL,false,NULL));
    struct mbuf *local_offer=NULL; assert(!sdp_encode(&local_offer,sdp,true));mem_deref(local_offer);
    assert(!session_alloc(&session,sdp,true,NULL,NULL,NULL));
    assert(!media_alloc(&opaque,session,NULL,NULL,NULL,NULL,NULL,media,NULL));
    struct menc_st *state=(struct menc_st *)opaque;
    struct mbuf *packet=mbuf_alloc(64); assert(packet);
    assert(!mbuf_write_u8(packet,128)); assert(!mbuf_write_u8(packet,0)); packet->pos=0;
    int error=0;
    assert(send_handler(&error,NULL,packet,state)); /* absolutely no pre-key RTP send */
    re_atomic_rlx_set(&state->got_sdp,true);
    assert(recv_handler(NULL,packet,state)); /* no plaintext delivery after SDP */
    struct pl key; pl_set_str(&key,"AAAA");
    assert(start_crypto(state,&key)); /* decoded-length mismatch must not report success */
    re_atomic_rlx_set(&state->use_srtp,false);
    packet->buf[1]=200; /* RTCP shares the mandatory gate */
    assert(send_handler(&error,NULL,packet,state)); assert(recv_handler(NULL,packet,state));
    const char *missing="v=0\r\no=peer 1 1 IN IP4 127.0.0.1\r\ns=test\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio 9002 RTP/SAVP 0\r\na=rtpmap:0 PCMU/8000\r\n";
    struct mbuf *offer=mbuf_alloc(1024); assert(offer); assert(!mbuf_write_str(offer,missing));offer->pos=0;
    assert(!sdp_decode(sdp,offer,false));
    assert(media_alloc(&opaque,session,NULL,NULL,NULL,NULL,NULL,media,NULL));
    assert(send_handler(&error,NULL,packet,state)); assert(recv_handler(NULL,packet,state));
    assert(media_txrekey(opaque)); /* cannot re-open a crypto-free negotiated stream */
    mbuf_reset(offer);assert(!mbuf_write_str(offer,missing));
    assert(!mbuf_write_str(offer,"a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\r\n"));offer->pos=0;
    assert(!sdp_decode(sdp,offer,false));assert(!media_alloc(&opaque,session,NULL,NULL,NULL,NULL,NULL,media,NULL));
    assert(re_atomic_rlx(&state->use_srtp) && state->srtp_tx && state->srtp_rx);
    assert(!media_txrekey(opaque));assert(re_atomic_rlx(&state->use_srtp));
    uint8_t rtp[12]={128,0,0,1,0,0,0,1,0,0,0,1};
    mbuf_reset(packet);assert(!mbuf_write_mem(packet,rtp,sizeof(rtp)));packet->pos=0;
    assert(recv_handler(NULL,packet,state)); /* plaintext stays rejected AFTER keys exist */
    mbuf_reset(packet);assert(!mbuf_write_mem(packet,rtp,sizeof(rtp)));packet->pos=0;
    error=0;assert(!send_handler(&error,NULL,packet,state));assert(!error);assert(mbuf_get_left(packet)==22);
    uint8_t gcm_key[28]={0};char encoded[64]={0};size_t encoded_len=sizeof(encoded)-1;
    assert(!base64_encode(gcm_key,sizeof(gcm_key),encoded,&encoded_len));encoded[encoded_len]=0;
    mbuf_reset(offer);assert(!mbuf_write_str(offer,missing));
    assert(!mbuf_printf(offer,"a=crypto:1 AEAD_AES_128_GCM inline:%s\r\n",encoded));offer->pos=0;
    assert(!sdp_decode(sdp,offer,false));assert(!media_alloc(&opaque,session,NULL,NULL,NULL,NULL,NULL,media,NULL));
    mbuf_reset(packet);assert(!mbuf_write_mem(packet,rtp,sizeof(rtp)));packet->pos=0;
    error=0;assert(!send_handler(&error,NULL,packet,state));assert(!error);assert(mbuf_get_left(packet)==28); /* TX changed suites too */
    mbuf_reset(offer);assert(!mbuf_write_str(offer,missing));
    assert(!mbuf_write_str(offer,"a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:AAAA\r\n"));offer->pos=0;
    assert(!sdp_decode(sdp,offer,false));assert(media_alloc(&opaque,session,NULL,NULL,NULL,NULL,NULL,media,NULL));
    assert(!re_atomic_rlx(&state->use_srtp));assert(send_handler(&error,NULL,packet,state));
    mem_deref(offer);
    mem_deref(packet); mem_deref(opaque); mem_deref(session); mem_deref(sdp);
    libre_close(); puts("mandatory SRTP pre-key RTP/RTCP and invalid key: PASS"); return 0;
}
