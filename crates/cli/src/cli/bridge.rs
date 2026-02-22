use anyhow::Result;
use clap::{Args, Subcommand};
use localgpt_server::BridgeManager;

#[derive(Args)]
pub struct BridgeArgs {
    #[command(subcommand)]
    pub command: BridgeCommands,
}

#[derive(Subcommand)]
pub enum BridgeCommands {
    /// Register a new bridge with credentials
    Register {
        /// Unique ID for the bridge (e.g., "telegram")
        #[arg(long)]
        id: String,

        /// Secret key/token for the bridge
        #[arg(long)]
        secret: String,
    },
}

pub async fn run(args: BridgeArgs) -> Result<()> {
    match args.command {
        BridgeCommands::Register { id, secret } => {
            let manager = BridgeManager::new();
            manager.register_bridge(&id, secret.as_bytes()).await?;
            // Note: Logging is handled by the core logging system, initialized in main.
            // But we can print to stdout for CLI feedback.
            println!("Bridge '{}' registered successfully.", id);
            println!("You may need to restart the daemon for changes to take effect.");
        }
    }
    Ok(())
}
