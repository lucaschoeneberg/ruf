const char *test_verified_host(const struct sip_msg *msg) {
    const struct test_peer *peer=msg ? msg->sock : NULL;
    return peer && peer->live ? peer->verified_host : NULL;
}
bool test_peer_matches(const struct sip_msg *msg,const void *socket,const char *verified_host) {
    const struct test_peer *peer=msg ? msg->sock : NULL;
    if (!msg || msg->tp!=SIP_TRANSP_TLS || !peer || !peer->live) return false;
    return verified_host ? peer->verified_host && !strcmp(peer->verified_host,verified_host) : socket && socket==msg->sock;
}
