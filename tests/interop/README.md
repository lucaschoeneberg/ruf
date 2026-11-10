# SIP interoperability

Start with the [isolated local Asterisk lab](asterisk/README.md). Its automated
smoke test verifies authenticated registration, one G.711 echo call, real returned
audio and clean teardown. Native SDK setup is in [native/README.md](../../native/README.md).

Expand compatibility only with observed, versioned test results. Asterisk success
does not establish compatibility with Ubiquiti, 3CX, FreePBX configurations or
other PBXs/providers. Public-network NAT, TLS/SRTP and hardware audio remain
separate test dimensions. See [recorded evidence](../../docs/verification/README.md#native-sip-and-audio).
