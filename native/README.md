# Native engine milestone

The foundation builds without baresip. This directory is reserved for reproducible
CMake recipes and exact baresip/libre source pins once the native milestone begins.
Build output belongs in ignored `build/` and `install/` directories here.

Follow upstream [build instructions](https://github.com/baresip/baresip#building)
and [library integration](https://github.com/baresip/baresip/wiki/Using-baresip-as-a-Library).
baresip uses CMake and depends on libre and OpenSSL. Select modules explicitly;
do not enable every optional codec or audio/video module.

Before linking, record exact revisions, licenses, platform audio drivers, TLS/SRTP
dependencies and artifact provenance. First prove registration, bidirectional audio
and clean shutdown on macOS arm64 and Windows x64. A successful scaffold build is
not evidence of SIP interoperability.
