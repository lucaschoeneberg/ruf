# Separate settings window and multiple accounts

User brief: keep the compact phone, open settings in a real second window,
follow the supplied Zed settings reference and Telephone account management.
Implement inline in the current checkout, reusing Svelte, Tauri commands,
ts-rs contracts and the existing baresip event loop. No new UI dependencies.

- [x] Extend password-free native snapshots with account IDs, names, per-account
      registration, selected outgoing account, call account and actual audio mode.
      Preserve existing configure_account for native smoke clients. Add meaningful
      tests for independent registrations, switching, removal and busy safeguards.
- [x] Extend the private C bridge to own up to eight baresip user agents, route
      callbacks by account ID, dial the chosen account and keep one global call slot.
      Reject account mutations during calls on both sides of the thread boundary.
- [x] Add explicit save/select/remove commands and a singleton settings window
      opened from Rust using async command execution (Windows WebView requirement).
      Give each window only its own IPC permissions; retain shared managed state.
- [x] Build Zed-style settings with searchable sidebar, account list, add/edit/
      remove, labels, validation and real registration feedback. Reuse onboarding
      validation and icons. Default audio remains microphone/speaker; global mode
      is read from native state rather than inferred from a webview draft.
- [x] Move setup out of the phone route and select outgoing accounts in its
      compact header. Existing onboarding opens the settings window.
- [x] Respect the user's pending choice about durable account storage. Existing
      behavior is memory-only; do not write passwords to frontend storage or files.
- [x] Document all commands, DTOs, limits and storage behavior. Verify portable
      and native Rust, generated types, Bun checks/build, actual Asterisk multi-account
      calls, native second-window lifecycle and bounded visual captures/review.

Review focus: no password readback; account ID survives asynchronous callbacks;
incoming calls belong to the correct UA; stale call actions remain isolated;
removal cannot destroy a live call; closing settings does not stop the engine.
