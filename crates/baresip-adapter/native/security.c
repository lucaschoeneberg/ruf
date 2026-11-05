/* OpenSSL performs TLS; native OS policy evaluates server trust without root exports. */
#include <re.h>
#include <openssl/ssl.h>
#include <openssl/x509v3.h>
#include <openssl/pem.h>
#include <string.h>
#include <stdio.h>
#include "security.h"
#ifdef __APPLE__
#include <Security/Security.h>
#elif defined(WIN32)
#include <windows.h>
#include <wincrypt.h>
#endif

static int identity(X509_STORE_CTX *ctx, char **owned_host) {
    SSL *ssl = X509_STORE_CTX_get_ex_data(ctx, SSL_get_ex_data_X509_STORE_CTX_idx());
    X509 *cert = X509_STORE_CTX_get0_cert(ctx);
    X509_VERIFY_PARAM *param = ssl ? SSL_get0_param(ssl) : NULL;
    const char *dns = param ? X509_VERIFY_PARAM_get0_host(param, 0) : NULL;
    char *ip = param && !dns ? X509_VERIFY_PARAM_get1_ip_asc(param) : NULL;
    int ok = 0;
    if (cert && dns && *dns) { ok = X509_check_host(cert, dns, 0, X509_CHECK_FLAG_NO_PARTIAL_WILDCARDS, NULL) == 1; *owned_host = OPENSSL_strdup(dns); }
    else if (cert && ip && *ip) { ok = X509_check_ip_asc(cert, ip, 0) == 1; *owned_host = OPENSSL_strdup(ip); }
    OPENSSL_free(ip);
    return ok && *owned_host;
}

/* Test mode uses a fresh OpenSSL store containing ONLY generated harness anchors. */
static int isolated_verify(X509_STORE_CTX *ctx, const char *ca) {
    X509_STORE *store = X509_STORE_new();
    X509_STORE_CTX *verify = X509_STORE_CTX_new();
    int ok = store && verify && X509_STORE_load_file(store, ca) == 1
        && X509_STORE_CTX_init(verify, store, X509_STORE_CTX_get0_cert(ctx), X509_STORE_CTX_get0_untrusted(ctx)) == 1;
    if (ok) { X509_STORE_CTX_set_purpose(verify, X509_PURPOSE_SSL_SERVER); ok = X509_verify_cert(verify) == 1; }
    X509_STORE_CTX_free(verify); X509_STORE_free(store);
    return ok;
}
#ifdef __APPLE__
static int platform_verify(X509_STORE_CTX *ctx, const char *host) {
    CFMutableArrayRef certs = CFArrayCreateMutable(NULL, 0, &kCFTypeArrayCallBacks);
    STACK_OF(X509) *chain = X509_STORE_CTX_get0_untrusted(ctx);
    SecPolicyRef policy = NULL; SecTrustRef trust = NULL;
    CFStringRef name = CFStringCreateWithCString(NULL, host, kCFStringEncodingUTF8);
    int ok = certs && name;
    int count = chain ? sk_X509_num(chain) : 0;
    /* Always put the leaf first; duplicate leaf in untrusted is harmless. */
    for (int i = -1; ok && i < count; ++i) {
        X509 *cert = i < 0 ? X509_STORE_CTX_get0_cert(ctx) : sk_X509_value(chain, i);
        unsigned char *der = NULL; int size = i2d_X509(cert, &der);
        CFDataRef data = size > 0 ? CFDataCreate(NULL, der, size) : NULL;
        SecCertificateRef native = data ? SecCertificateCreateWithData(NULL, data) : NULL;
        OPENSSL_free(der); if (data) CFRelease(data);
        if (!native) ok = 0; else { CFArrayAppendValue(certs, native); CFRelease(native); }
    }
    if (ok) policy = SecPolicyCreateSSL(true, name);
    if (ok) ok = policy && SecTrustCreateWithCertificates(certs, policy, &trust) == errSecSuccess;
    /* Keep SIP event processing bounded; only supplied/keychain/cached intermediates. */
    if (ok) ok = SecTrustSetNetworkFetchAllowed(trust, false) == errSecSuccess;
    if (ok) ok = SecTrustEvaluateWithError(trust, NULL);
    if (trust) CFRelease(trust); if (policy) CFRelease(policy);
    if (name) CFRelease(name); if (certs) CFRelease(certs);
    return ok;
}
#elif defined(WIN32)
static int platform_verify(X509_STORE_CTX *ctx, const char *host) {
    HCERTSTORE intermediates = CertOpenStore(CERT_STORE_PROV_MEMORY, 0, 0, CERT_STORE_CREATE_NEW_FLAG, NULL);
    PCCERT_CONTEXT leaf = NULL; PCCERT_CHAIN_CONTEXT chain = NULL;
    STACK_OF(X509) *untrusted = X509_STORE_CTX_get0_untrusted(ctx);
    WCHAR name[256]; int ok = intermediates && MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, host, -1, name, 256);
    int count = untrusted ? sk_X509_num(untrusted) : 0;
    for (int i=-1; ok && i<count; ++i) {
        unsigned char *der=NULL; int size=i2d_X509(i<0 ? X509_STORE_CTX_get0_cert(ctx) : sk_X509_value(untrusted,i), &der);
        PCCERT_CONTEXT cert=size>0 ? CertCreateCertificateContext(X509_ASN_ENCODING, der, (DWORD)size) : NULL;
        OPENSSL_free(der);
        if (!cert) { ok=0; break; }
        if (i<0) leaf=cert;
        else { ok=CertAddCertificateContextToStore(intermediates,cert,CERT_STORE_ADD_ALWAYS,NULL); CertFreeCertificateContext(cert); }
    }
    LPSTR usage=szOID_PKIX_KP_SERVER_AUTH;
    CERT_CHAIN_PARA para={0}; para.cbSize=sizeof(para); para.RequestedUsage.dwType=USAGE_MATCH_TYPE_AND;
    para.RequestedUsage.Usage.cUsageIdentifier=1; para.RequestedUsage.Usage.rgpszUsageIdentifier=&usage;
    if (ok) ok=CertGetCertificateChain(NULL,leaf,NULL,intermediates,&para,CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL,NULL,&chain);
    SSL_EXTRA_CERT_CHAIN_POLICY_PARA extra={0}; extra.cbSize=sizeof(extra); extra.dwAuthType=AUTHTYPE_SERVER; extra.pwszServerName=name;
    CERT_CHAIN_POLICY_PARA policy={0}; policy.cbSize=sizeof(policy); policy.pvExtraPolicyPara=&extra;
    CERT_CHAIN_POLICY_STATUS status={0}; status.cbSize=sizeof(status);
    if (ok) ok=CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_SSL,chain,&policy,&status) && status.dwError==0;
    if (chain) CertFreeCertificateChain(chain); if (leaf) CertFreeCertificateContext(leaf); if (intermediates) CertCloseStore(intermediates,0);
    return ok;
}
#else
static int platform_verify(X509_STORE_CTX *ctx, const char *host) {
    X509_STORE *store=X509_STORE_new(); X509_STORE_CTX *verify=X509_STORE_CTX_new();
    (void)host;
    int loaded=store && X509_STORE_set_default_paths(store);
    const char *files[]={"/etc/ssl/certs/ca-certificates.crt","/etc/pki/tls/certs/ca-bundle.crt","/etc/ssl/ca-bundle.pem"};
    for (size_t i=0;store && i<sizeof(files)/sizeof(files[0]);++i) if (X509_STORE_load_file(store,files[i])==1) loaded=1;
    int ok=loaded && verify && X509_STORE_CTX_init(verify,store,X509_STORE_CTX_get0_cert(ctx),X509_STORE_CTX_get0_untrusted(ctx))==1;
    if (ok) { X509_STORE_CTX_set_purpose(verify,X509_PURPOSE_SSL_SERVER); ok=X509_verify_cert(verify)==1; }
    X509_STORE_CTX_free(verify); X509_STORE_free(store); return ok;
}
#endif
static int verify_server(void *opaque, void *arg) {
    X509_STORE_CTX *ctx=opaque; char *host=NULL;
    int ok=identity(ctx,&host);
    if (ok) ok=arg ? isolated_verify(ctx,arg) : platform_verify(ctx,host);
    OPENSSL_free(host);
    X509_STORE_CTX_set_error(ctx, ok ? X509_V_OK : X509_V_ERR_CERT_REJECTED);
    return ok;
}
int sip_security_configure(struct tls *tls, const char *ca_file) {
    if (!tls) return EINVAL;
    return tls_set_server_verify_handler(tls, verify_server, (void *)ca_file);
}
