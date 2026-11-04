# Licensing and attribution

## Project and contributions

Original application code and original project documentation use the
[MIT license](../LICENSE), identified as `MIT` in SPDX, Cargo and npm metadata.
The standard text is preserved. Community/contribution rules live in separate
documents rather than being inserted into LICENSE.

MIT permits use, modification and redistribution, including commercial use. Copies
or substantial portions must retain the copyright and permission notice. The
license includes a warranty disclaimer and liability limitation.
[SPDX text](https://spdx.org/licenses/MIT.html),
[MIT overview](https://choosealicense.com/licenses/mit/).

`Copyright (c) 2026 SIP contributors` is the project's collective notice.
Contributors retain copyright in their work; this does not assign it to a maintainer
or claim one person owns all contributions. Preserve existing copyright notices.

Original contributions intentionally submitted for inclusion are accepted under
the same MIT terms, as stated in [CONTRIBUTING](../CONTRIBUTING.md). No separate
CLA, DCO sign-off or cryptographic signature is required. Contributors must have
the right to grant those permissions. Clearly marked third-party material is
reviewed separately under its original license.

## Dependencies, copied material and assets

The project's MIT license does not relicense dependencies. Before adding a library
or importing code, fixtures, documentation, fonts, icons or other assets:

1. Record upstream source, version/revision and imported files.
2. Read the applicable license, including bundled code and optional modules.
3. Preserve copyright, license and required NOTICE/attribution material.
4. Explain modifications and update
   [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) as appropriate.
5. Check that the intended source/binary distribution can meet the applicable terms.

Prefer suitable permissive dependencies. They need not all be MIT: BSD and
Apache-2.0 components retain their own conditions and notices. Discuss copyleft
or unusual terms before inclusion; open source does not mean material can be
redistributed as MIT. Do not import material with unresolved rights or license terms.

baresip is BSD-3-Clause and is not linked/bundled in this scaffold. The native
milestone must review the actual baresip/libre/OpenSSL versions and selected
modules/codecs rather than infer every module's license from the top-level project.
[baresip license](https://github.com/baresip/baresip/blob/main/LICENSE)

## Distribution

Ship LICENSE and relevant third-party texts/notices with artifacts. Resolve the
inventory from the actual locked packages, platform and enabled native modules;
lockfiles identify versions but are not license notices. Check asset provenance
and keep SPDX/package metadata consistent.

The scaffold bundles its own MIT license and current THIRD_PARTY_NOTICES. That
file is explicitly not a complete transitive inventory. Complete distribution
review remains necessary before a public production release.
