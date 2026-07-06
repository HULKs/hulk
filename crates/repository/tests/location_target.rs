use clap::ValueEnum;
use repository::location::LocationTarget;

#[test]
fn location_target_exposes_default_target_behavior() {
    assert_eq!(LocationTarget::all(), [LocationTarget::Default]);
    assert_eq!(LocationTarget::Default.to_string(), "default");
    assert_eq!(LocationTarget::Default.file_name(), "default_location");
    assert_eq!(
        LocationTarget::from_str("default", true),
        Ok(LocationTarget::Default)
    );
}
