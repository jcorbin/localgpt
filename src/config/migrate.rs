//! OpenClaw detection
//!
//! Checks for an existing OpenClaw installation and returns a notice
//! directing users to the migration guide.

/// Check if an OpenClaw data directory exists at ~/.openclaw.
///
/// Returns a formatted notice string if detected, `None` otherwise.
pub fn check_openclaw_detected() -> Option<String> {
    let base = directories::BaseDirs::new()?;
    let openclaw_dir = base.home_dir().join(".openclaw");

    if openclaw_dir.exists() {
        Some(
            "Note: OpenClaw data detected at ~/.openclaw\n  \
             Migration guide: https://localgpt.app/docs/openclaw-migration"
                .to_string(),
        )
    } else {
        None
    }
}
