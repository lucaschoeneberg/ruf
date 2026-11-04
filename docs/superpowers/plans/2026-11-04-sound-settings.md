# Sound settings

User confirmed: microphone and speaker selection, independent ringtone output,
ringtone enabled/disabled, and audible preview. Preserve separate Zed-style
settings window; no volume/microphone-meter extension in this task.

- Reuse baresip's CoreAudio (macOS) and WASAPI (Windows) backends and player.
  AudioUnit ignores explicit devices, so replace it with upstream CoreAudio.
- Add generated SoundSettings/AudioDevices contracts; default devices are empty
  IDs. Store nonsecret preferences in existing versioned metadata with defaults
  for old files; never initialize microphones just by reading settings.
- Query real native devices without credentials/account creation. Validate and
  apply preferences on the existing event loop while idle, including a second
  call-state guard in C. Acknowledgments prevent false success. Persist settings
  and reuse selections on subsequent calls/restarts; unavailable devices remain
  visible with recovery instead of silently changing them.
- Generate a short native PCM ring, play through baresip, loop for incoming calls,
  stop on answer/reject/close/shutdown; preview only while idle and stop it when
  incoming/outgoing calls begin. Test-tone lab sessions never ring automatically.
- Build a Ton panel using existing row controls, status/errors and explicit Save;
  reveal/focus unsaved confirmation on navigation. Put developer test source in
  secondary advanced content. No second frontend audio pipeline/dependencies.
- Test validation/persistence, queue guards/native settings, fixture UI behavior,
  actual macOS device enumeration/preview lifecycle, Asterisk regression; run
  full existing quality checks, native build and independent bounded review.

Completed: native inventory/routing/PCM preview, persistent sound defaults,
Ton panel and draft protection, API documentation, macOS SDK rebuild, actual
playback checks, browser fixtures and independent code/visual review. macOS
duplicate names are explicitly unselectable. Windows runtime and microphone
capture remain unverified; no volume/meter scope was added.
