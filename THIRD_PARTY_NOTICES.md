# Third-party notices

Own application code is MIT licensed. Rust/JavaScript dependencies retain the licenses
recorded in their packages; lockfiles identify resolved versions. This file is not
a completed inventory of transitive dependency licenses.

For incoming contributions, copied assets and distribution requirements, see
[licensing and attribution](docs/licensing.md). The project's MIT license does not
replace third-party licenses.

The temporary desktop icons and initial Tauri configuration conventions originate
from [create-tauri-app](https://github.com/tauri-apps/create-tauri-app), distributed
under MIT or Apache-2.0. Used under MIT:

Copyright (c) 2019 - 2022 Tauri Programme within The Commons Conservancy

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in the
Software without restriction, including without limitation the rights to use,
copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the
Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Native-feature builds statically link the following upstream components. Their
unmodified license texts are in `native/licenses/` and included as `native-licenses`
in desktop bundles. Own application code remains MIT.

| Component                             | Version/revision                                                         | License and source                                                                                     |
| ------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| baresip, selected G.711/audio modules | 4.12.0 / `9cfab81550acc4bd38d6580d79b45cc6eea65cd6`                      | BSD-3-Clause; [source](https://github.com/baresip/baresip), [license](native/licenses/baresip-LICENSE) |
| libre/re (including rem)              | 4.12.0 / `65555dad439de9d6a81b3ee6092b9c096585249b`                      | BSD-3-Clause; [source](https://github.com/baresip/re), [license](native/licenses/re-LICENSE)           |
| OpenSSL                               | Host-resolved 3.x; exact version recorded in SDK manifest (local: 3.5.9) | Apache-2.0; [source](https://github.com/openssl/openssl), [license](native/licenses/openssl-LICENSE)   |

The libre SDK includes a local [SIP NAT patch](native/patches/README.md) applied
to the exact revision above. Upstream notices and license text are preserved.
The baresip SDK also includes a minimal tracked
[REFER-policy patch](native/patches/baresip-refer-policy.patch) enforcing its
existing disabled-transfer setting. Its original BSD notices are preserved.

The native build also stages license texts from the actual SDK inputs. Windows
OpenSSL resolution is constrained by `native/vcpkg.json`; macOS/Linux system
packages and toolchains remain external inputs. The selected modules are recorded
in `sdk.json`; no third-party audio codec packages are added in this milestone.
This is not a complete transitive/source-file/optional-module license inventory.
Complete that review before a production distribution.

The digest-pinned Asterisk community container is a separate local/CI test tool,
not bundled with the app. Asterisk is GPL-2.0; its distribution and accompanying
third-party components retain their own upstream terms. See the
[lab guide](tests/interop/asterisk/README.md) and
[Asterisk license](https://github.com/asterisk/asterisk/blob/master/LICENSE).

Saved desktop credentials use [keyring](https://github.com/open-source-cooperative/keyring-rs)
4.2.0 and keyring-core 1.0.1, with apple-native-keyring-store 1.0.2 on macOS and
windows-native-keyring-store 1.1.0 on Windows. These crates are used under their MIT
license option. Their upstream MIT notices are included in `native/licenses/`
and bundled with the desktop application's existing license resources. Other
transitive Rust components retain the licenses recorded in their packages.

The interface bundles unmodified IBM Plex Sans Regular and SemiBold from
[Zed's font assets](https://github.com/zed-industries/zed/tree/e21ff1b368b074586e4dae0d9bbf15d5dd8a1476/assets/fonts/ibm-plex-sans).
Copyright © 2017 IBM Corp., Reserved Font Name "Plex". The fonts use the
SIL Open Font License 1.1, reproduced in
[IBM-Plex-Sans-LICENSE.txt](apps/desktop/public/fonts/IBM-Plex-Sans-LICENSE.txt).
That notice and both font files are distributed with the frontend assets.
Application code remains under MIT.

The shared dark interface palette references the installed GitHub Dark Tritanopia
variant of [GitHub Zed Themes](https://github.com/PyaeSoneAungRgn/github-zed-theme),
which is MIT licensed. Its [upstream notice](native/licenses/github-zed-theme-LICENSE-MIT)
is bundled with the existing native license resources.

Native background integration uses official Tauri plugins under their MIT option:
`tauri-plugin-autostart` 2.7.0,
`tauri-plugin-single-instance` 2.5.5 and `tauri-plugin-dialog` 2.8.4. The
macOS startup keychain UI guard reuses `security-framework` 3.7.0 under MIT.
Their unmodified MIT texts are staged in `native/licenses/` and included in bundles.
The tray handset is drawn by our application code; no additional icon asset is copied.

Actionable call notifications use our native UserNotifications.framework bridge
on macOS and the Microsoft `windows`/`windows-core` 0.62.2 crates on Windows.
Opaque offer tokens use `getrandom` 0.4.3. These Rust crates are used under their
MIT license option; their unmodified notices are staged in `native/licenses/`.
The operating-system frameworks are platform components, not copied source.

Local call history uses rusqlite 0.41.1 and libsqlite3-sys 0.39.1 under MIT.
Their unmodified upstream notices are bundled in `native/licenses/`. The bundled
SQLite engine is dedicated to the public domain by its authors; see the notice
in the bundled sqlite3 source and [SQLite copyright](https://sqlite.org/copyright.html).
