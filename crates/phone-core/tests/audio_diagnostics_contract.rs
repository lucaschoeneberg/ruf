//! Default-preserving audio preferences and bounded device fields.
use phone_core::{ErrorCode, Ringtone, SoundSettings};
#[test]
fn ringtone_defaults_preserve_original_amplitude_and_validate_volume() {
    let mut settings = SoundSettings::default();
    assert_eq!(settings.ringtone, Ringtone::Classic);
    assert_eq!(settings.ringtone_volume, 100);
    settings.ringtone_volume = 101;
    assert_eq!(
        settings.validate().unwrap_err().code,
        ErrorCode::InvalidInput
    );
    for volume in [0, 1, 50, 100] {
        settings.ringtone_volume = volume;
        assert!(settings.validate().is_ok());
    }
}
