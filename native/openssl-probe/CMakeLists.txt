cmake_minimum_required(VERSION 3.22)
project(sip_openssl_probe C)
set(OPENSSL_USE_STATIC_LIBS TRUE)
find_package(OpenSSL 3 REQUIRED)
# Imported target paths resolve MSVC's configuration-specific library names too.
file(GENERATE OUTPUT "${CMAKE_BINARY_DIR}/openssl-$<CONFIG>.json"
  CONTENT "{\"ssl\":\"$<TARGET_FILE:OpenSSL::SSL>\",\"crypto\":\"$<TARGET_FILE:OpenSSL::Crypto>\",\"version\":\"${OPENSSL_VERSION}\"}\n")
