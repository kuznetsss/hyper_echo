use std::io::IsTerminal;

use clap::Parser;
use tracing::{info, Level};

use hyper_echo::EchoServer;

#[derive(Debug, Parser)]
#[command(about = "A simple echo server with http and websocket support")]
struct Args {
    /// Set log level for requests and responses: 0 - no logging, 1 - log uri, 2 - log uri and headers, 3 - log uri, headers and body
    #[arg(short('l'), long, default_value = "0", value_parser = clap::value_parser!(u8).range(0..=3))]
    http_log_level: u8,

    /// Log websocket messages
    #[arg(short('w'), long, action)]
    log_ws: bool,

    /// Verbose logging. A shortcut for --http_log_level=3 --log_ws
    #[arg(short, long, action)]
    verbose: bool,

    /// Threads number
    #[arg(short, long, default_value = "1")]
    threads: usize,

    /// Port for the server (a random free port will be used if not provided)
    #[arg(short, long)]
    port: Option<u16>,
}

impl Args {
    fn parse() -> Self {
        let mut args = <Args as Parser>::parse();
        if args.verbose {
            args.log_ws = true;
            args.http_log_level = 3;
        }
        args
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_ansi(std::io::stdout().is_terminal())
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let args = Args::parse();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(args.threads)
        .build()
        .unwrap()
        .block_on(async move {
            let echo_server = EchoServer::new(args.port, args.http_log_level.into(), args.log_ws).await?;
            info!("Starting echo server on {}", echo_server.local_addr());
            echo_server.run().await
        })
        .map_err(Into::into)
}
