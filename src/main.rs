use clap::{Parser, Subcommand};

/// agdog: agent-aware terminal resource monitor.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Subscribe to the event socket and print each event as JSON.
    Watch,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Watch) => {
            agdog::socket::watch(agdog::socket::socket_path())?;
            Ok(())
        }
        None => agdog::app::run(),
    }
}
