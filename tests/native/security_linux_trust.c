/* Execute the actual Linux verifier branch without changing the host trust store. */
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <openssl/ssl.h>
#include <openssl/pem.h>
#include <openssl/x509v3.h>
#include LINUX_TRUST_SOURCE
int main(int argc,char **argv) {
 assert(argc==4);BIO *input=BIO_new_file(argv[2],"r");assert(input);X509 *leaf=PEM_read_bio_X509(input,NULL,NULL,NULL);BIO_free(input);assert(leaf);
 for(int test=0;test<2;++test) {
  assert(!setenv("SSL_CERT_FILE",test ? "/nonexistent/ruf-test" : argv[1],1));
  assert(!setenv("SSL_CERT_DIR",test ? argv[3] : "/nonexistent/ruf-test",1));
  X509_STORE *store=X509_STORE_new();X509_STORE_CTX *ctx=X509_STORE_CTX_new();assert(store&&ctx);
  assert(X509_STORE_CTX_init(ctx,store,leaf,NULL));
  assert(!platform_verify(ctx,"localhost"));
  X509_STORE_CTX_free(ctx);X509_STORE_free(store);
 }
 X509_free(leaf);puts("Linux production verifier ignores SSL_CERT_FILE/DIR anchors: PASS");return 0;
}
