//! Opt-in native device and audible preview check. Never opens microphone capture.
use baresip_adapter::{Engine, audio_devices};
use phone_core::{AudioMode, SoundSettings};
use std::{
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = audio_devices()?;
    println!(
        "AUDIO_DEVICES inputs={} outputs={}",
        devices.inputs.len(),
        devices.outputs.len()
    );
    let mut engine = Engine::start(AudioMode::System, None)?;
    let original = SoundSettings::default();
    engine.set_sound_settings(&original)?;
    let output = devices
        .outputs
        .iter()
        .find(|device| device.selectable)
        .ok_or("No output device for audible test")?
        .id
        .clone();
    let settings = SoundSettings {
        output_device: output.clone(),
        ring_device: output.clone(),
        ringtone_enabled: false,
        ..original.clone()
    };
    engine.set_sound_settings(&settings)?;
    assert_eq!(engine.snapshot()?.sound_settings, settings);
    let invalid = SoundSettings {
        output_device: "missing-test-device".into(),
        ..settings.clone()
    };
    assert!(engine.set_sound_settings(&invalid).is_err());
    assert_eq!(engine.snapshot()?.sound_settings, settings);
    engine.preview_ringtone(&output)?;
    assert!(engine.snapshot()?.ringtone_playing);
    engine.stop_ringtone_preview()?;
    assert!(!engine.snapshot()?.ringtone_playing);
    engine.preview_ringtone(&output)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while engine.snapshot()?.ringtone_playing {
        assert!(Instant::now() < deadline, "Preview must stop automatically");
        thread::sleep(Duration::from_millis(20));
    }
    engine.shutdown()?;
    println!(
        "SOUND_OK routing, invalid-device rejection, explicit stop, automatic preview completion"
    );
    Ok(())
}
