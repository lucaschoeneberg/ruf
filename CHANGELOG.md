# What's new in Ruf?

## 0.1.0 · Preview

- Search Ruf with ⌘K on Mac or Ctrl+K on Windows for actions, authorized contacts and past calls. Results open existing details; a call starts only through its explicit call action. Keyboard shortcuts and native menus provide direct access to call controls.
- Dial formatted numbers with spaces and a leading plus, and use contact suggestions as you type.
- Show the in-call keypad for digits, star and hash; tones are sent natively through baresip.
- Use current contact names for outgoing and incoming calls, notifications and older history entries, with optional German national and international number matching.

- Mute and hold calls natively, with separate indicators for remote hold and pending changes.
- Transfer directly or start a consultation, switch between both calls, return to the original party and complete the transfer. Success follows native confirmation; incoming REFER instructions remain rejected.
- Save UDP/TCP/TLS transport, required SDES-SRTP with TLS, a separate authentication username and an outbound proxy per account. Server verification uses operating-system trust without automatic plaintext fallback. Displayed call security follows native observations and does not guarantee end-to-end encryption.

- Manage up to eight telephone accounts, enable or disable each independently, and select the outgoing account.
- Use eight settings sections with field search, keyboard navigation and protection against losing unsaved changes.
- Reconnect saved accounts after a pending Keychain authorization using the reconnect action.
- Select microphone, call speaker and ringtone output separately; choose among three ringtones and adjust ringtone volume.
- Test the microphone with a short level meter and the speaker with a two-second tone. Incoming calls stop active audio tests.
- Check notification, microphone and contacts permissions, then explicitly request access or open system settings as needed.
- Answer or reject incoming calls in the background through system notifications, with optional caller-detail hiding.
- Request a neutral test notification after five seconds. Focus and system settings can still suppress its display.
- Start Ruf at login with or without a visible phone window; manually opening the app keeps the window visible.
- Search and call macOS contacts after permission, and configure favorite ordering and company display.
- View past calls with their outcomes and duration, and call back through the original account.
- Disable saving future calls to history or remove completed entries after 30, 90 or 365 days; unlimited retention remains the default.
- Distinguish caller information, call states and SIP errors more precisely. Unsupported transfer requests are explicitly rejected.
- View and explicitly copy a diagnostic report without personal account, contact or caller data.
- Migrate existing settings while backing up the original metadata, with improved long account names, clear account actions and search focus.
