use clap::{Parser, Subcommand};

/// agdog: agent-aware terminal resource monitor.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Refresh interval in seconds.
    #[arg(long, default_value_t = 1)]
    interval: u64,

    /// GPU cost rate in dollars per hour, used to derive per-agent cost.
    #[arg(long, default_value_t = 0.0)]
    gpu_hourly: f64,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Subscribe to the event socket and print each event as JSON.
    Watch,
    /// Print the attributed agents once and exit (for debugging attribution).
    Agents,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Watch) => {
            agdog::socket::watch(agdog::socket::socket_path())?;
            Ok(())
        }
        Some(Command::Agents) => agdog::app::dump_agents(),
        None => agdog::app::run(cli.interval, cli.gpu_hourly),
    }
}
