/* Native-only trust boundary; never changes OS trust stores. */
#ifndef SIP_SECURITY_H
#define SIP_SECURITY_H
struct tls;
/* ca_file is NULL for platform policy; non-NULL is exclusive test trust. */
int sip_security_configure(struct tls *tls, const char *ca_file);
#endif
