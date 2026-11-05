# Notification and background reliability

Observed: the installed Ruf process has active phone, SIP and notification
threads. Native notification delivery works in the isolated OS test. The UI
collapses undecided and denied authorization; automatic login setup can wait
behind a foreground permission request; failed delivery is permanently latched
for the current call and native service waits have no deadline.

- Preserve the existing permission boolean and add the native four-state status
  through ts-rs. Show the appropriate explicit request or OS-settings guidance;
  refresh after window focus without replacing unsaved preferences.
- Make observer login setup defer rather than block on foreground preferences.
- Bound native settings/delivery waits and retry failed call delivery with a
  bounded backoff. Keep SIP and action validation independent of OS latency.
- Add failing regressions, document the contract, run native OS notification,
  SIP, frontend and Rust checks, independently review and rebuild Ruf.

Reuse the existing UserNotifications delegate, latest-state worker, Tauri
commands, preferences lock, call-token registry and generated DTOs. No new
dependency, duplicate SIP worker or personal account access.

Implemented and verified: see
[notification verification](../../verification/2026-11-05-notification-reliability.md)
and [startup incoming evidence](../../verification/2026-11-05-startup-incoming.md).
Real-provider fresh-launch/idle acceptance remains outstanding; the isolated
startup test passes without a preceding outgoing call.
