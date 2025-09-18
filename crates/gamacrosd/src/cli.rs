use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Subcommand, PartialEq)]
pub(crate) enum ControlCommand {
    /// Rumble the controller
    Rumble {
        /// The controller ID to rumble
        #[clap(short, long)]
        id: Option<u32>,
        /// The duration of the rumble in milliseconds
        #[clap(short, long)]
        ms: u32,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Subcommand, PartialEq)]
pub(crate) enum Command {
    /// Run the daemon in the foreground.
    Run {
        /// The profile to run
        #[clap(short, long)]
        workspace: Option<String>,
    },
    /// Start daemon in the background.
    Start {
        /// The directory containing the profile
        #[clap(short, long)]
        workspace: Option<String>,
    },
    /// Stop the daemon.
    Stop,
    /// Show the status of the daemon.
    Status,
    /// Observe the daemon's events.
    Observe,
    /// Send a command to the daemon.
    Command {
        /// The workspace to send the command to
        #[clap(short, long)]
        workspace: Option<String>,
        /// The command to send
        #[clap(subcommand)]
        command: ControlCommand,
    },
}

/// Highly effective conversion of a gamepad into a macropad for applications.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Turn debugging information on
    #[arg(short, long)]
    pub verbose: bool,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// The command to run
    #[clap(subcommand)]
    pub command: Command,
}
