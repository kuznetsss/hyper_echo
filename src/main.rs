use std::io::IsTerminal;

use clap::Parser;
use tracing::{info, Level};

use echo_server::EchoServer;

#[derive(Debug, Parser)]
struct Args {
    /// Print requests and responses
    #[arg(short, long, default_value = "false")]
    logging_enabled: bool,

    /// Threads number
    #[arg(short, long, default_value = "1")]
    threads: usize,

    /// Port for the server (use 0 for a random free port)
    #[arg(short, long, default_value = "0")]
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_ansi(std::io::stdout().is_terminal())
        .with_max_level(Level::INFO)
        .init();

    let args = Args::parse();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(args.threads)
        .build()
        .unwrap()
        .block_on(async move {
            let echo_server = EchoServer::new(args.logging_enabled, args.port).await?;
            info!("Starting echo server on {}", echo_server.local_addr());
            echo_server.run().await
        })
        .map_err(Into::into)
}
