# Licensing and attribution

## Project and contributions

Original application code and original project documentation use the
[MIT license](../LICENSE), identified as `MIT` in SPDX, Cargo and JavaScript package metadata.
The standard text is preserved. Community/contribution rules live in separate
documents rather than being inserted into LICENSE.

MIT permits use, modification and redistribution, including commercial use. Copies
or substantial portions must retain the copyright and permission notice. The
license includes a warranty disclaimer and liability limitation.
[SPDX text](https://spdx.org/licenses/MIT.html),
[MIT overview](https://choosealicense.com/licenses/mit/).

`Copyright (c) 2026 Ruf contributors` is the project's collective notice.
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

Native builds link baresip and libre 4.13.0 (BSD-3-Clause) plus host-resolved
OpenSSL 3.x (Apache-2.0). Exact source pins, selected modules and license texts
are recorded under `native/` and in THIRD_PARTY_NOTICES. The SDK records the
resolved OpenSSL version. Complete the source-file/transitive-module inventory
before production distribution; do not infer every embedded component's license
from a top-level project.
[baresip license](https://github.com/baresip/baresip/blob/main/LICENSE)

## Distribution

Ship LICENSE and relevant third-party texts/notices with artifacts. Resolve the
inventory from the actual locked packages, platform and enabled native modules;
lockfiles identify versions but are not license notices. Check asset provenance
and keep SPDX/package metadata consistent.

The desktop bundles its MIT license, current THIRD_PARTY_NOTICES and tracked
native license texts. That
file is explicitly not a complete transitive inventory. Complete distribution
review remains necessary before a public production release.

## About page and project SBOM

The About Ruf section in Settings identifies **Mardu GmbH** as Ruf's author/developer and
links to its website, legal notice and website privacy policy. This attribution
does not replace the project's collective copyright notice or third-party rights.

The page embeds the existing LICENSE, THIRD_PARTY_NOTICES, native license files
and IBM Plex Sans license as plain text, so they remain readable offline.
The initially collapsed software overview shows only components present in the
selected build (at most eight central entries), with locked/SDK versions. The
complete machine-readable graph remains a separate nested disclosure. Company
attribution and links stay in the small footer.

The Vite build resolves Rust dependencies through `cargo metadata --locked` for
one target and the `native` feature choice. Non-development Cargo dependencies
are traversed from the desktop package; proc-macro and build-only subgraphs use
CycloneDX `scope: excluded`, while application dependencies use `required`.
JavaScript components and cross-package import edges come from the rendered
Vite module graph and installed package manifests, rather than every entry in
`bun.lock`. Bundled IBM Plex font files include SHA-256 hashes and their license.

Native builds require a matching SDK. Source pins and local patch texts must
match the tracked inputs. The inventory includes baresip, libre, the SDK's exact
OpenSSL version, selected modules, patch hashes and static archive hashes.
Portable builds omit those three native libraries. OS-provided system libraries
are outside the packaged-component inventory. No additional third-party package is introduced;
the build script reuses the existing serde_json crate for build validation.

`bun run desktop:build` selects one concrete target, profile and native mode,
checks pinned toolchain versions, and passes the same context to Vite/Tauri.
The Rust build script rejects a packaged SBOM whose target, profile, native mode
or frontend-resolution phase differs from the actual Cargo build. It embeds a
snapshot of the validated JSON in the executable. The settings-only
`get_software_bill` command and the non-GUI CLI export read that exact snapshot.
The compiled SBOM carries a timestamp, generator, target/mode/profile, lockfile
hashes and explicit dependency references. Rebuild after inputs change.

After packaging, the wrapper invokes `--export-sbom <path>` before any app/SIP
startup and emits `Ruf_<version>_<target>_<native|portable>.cdx.json` plus a
`.build.json` containing SBOM, executable and newly produced installer hashes.
They are placed in the target/profile directory and alongside the installers.
The artifact and draft-release workflows retain these files. They do not publish
an installer or establish regulatory conformity by themselves.

Use the wrapper for desktop builds, including `--debug`, and `bun run dev:native`
for native development. Direct release Cargo/Tauri builds without a matching
packaged inventory fail. Universal macOS binaries are not supported by this
single-target SBOM path; build and inventory each concrete target separately.
Ordinary Cargo debug checks remain possible without a frontend build. Standalone
Vite builds and dev-server inventories are explicitly marked as previews; they
are not a substitute for the inventory embedded in a desktop release.

License/patch provenance and dependency coverage remain subject to distribution
review, especially upstream embedded source components. Resolving the concrete
SBOM defects is not a blanket declaration of compliance with the entire CRA.
