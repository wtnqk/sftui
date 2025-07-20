use anyhow::Result;
use clap::Parser;

mod app;
mod sftp;
mod ssh_config;
mod ui;

use app::App;

#[derive(Parser, Debug)]
#[command(name = "sftui")]
#[command(about = "A TUI SFTP client")]
struct Args {
    #[arg(short = 'H', long, help = "SSH host to connect to")]
    host: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut app = App::new(args.host).await?;
    app.run().await?;

    Ok(())
}

