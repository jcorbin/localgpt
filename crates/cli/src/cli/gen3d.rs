//! CLI subcommand for LocalGPT Gen — 3D scene generation mode.
//!
//! TODO: Gen mode has been moved to a separate `localgpt-gen` crate.
//! This stub preserves the GenArgs struct for the CLI enum but the actual
//! implementation now lives in the gen crate. When the "gen" feature is
//! enabled, the gen crate should be pulled in as a dependency.

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct GenArgs {
    /// Initial prompt to send (optional — starts interactive if omitted)
    pub prompt: Option<String>,
}

/// Launch LocalGPT Gen: Bevy window on main thread, agent on background tokio runtime.
///
/// TODO: This function needs to be wired up to the `localgpt-gen` crate once it
/// is added as a dependency with the "gen" feature flag.
pub fn run(_args: GenArgs, _agent_id: &str) -> Result<()> {
    anyhow::bail!(
        "Gen mode is not available in the CLI crate. \
         The gen3d module has been moved to the separate localgpt-gen crate."
    )
}
