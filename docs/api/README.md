# Desktop IPC API

## get_app_info

Read-only command allowed for the `main` window. No request body or optional
parameters. No network access, credential access, database writes or engine startup.

```ts
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "../../apps/desktop/src/lib/generated/AppInfo";

const info = await invoke<AppInfo>("get_app_info");
```

Response example:

```json
{ "version": "0.1.0", "telephony": "notLinked" }
```

| Field       | Type                       | Meaning                                                       |
| ----------- | -------------------------- | ------------------------------------------------------------- |
| `version`   | required string            | Compiled application version, aligned with the bundle version |
| `telephony` | required `TelephonyStatus` | Native engine availability; currently only `notLinked`        |

`notLinked` means no native baresip library is linked. It does not describe account
registration and must never be presented as ready to make calls.

There are no application-domain errors in this command yet. Invocation can reject
if the runtime is unavailable, IPC fails or permission is denied. `getAppInfo()` in
the frontend rejects explicitly in browser preview; the UI displays that error.
Tauri IPC does not use HTTP status codes. Future calling errors and SIP response
codes must be documented independently.

## Contract generation

`crates/phone-core/src/lib.rs` is the source of truth. Generate with
`npm run types:generate`, commit the resulting TypeScript, and inspect its diff.
CI regenerates and rejects tracked changes. Generated files are excluded from
Prettier so the generator owns their formatting. Adding a public DTO requires
updating the exporter and this document together.
