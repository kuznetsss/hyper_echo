use std::{io::IsTerminal, process::exit};

use clap::Parser;
use tokio::{select, signal::ctrl_c};
use tokio_util::sync::CancellationToken;
use tracing::{Level, info};

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

    /// Websocket ping interval in milliseconds
    #[arg(short('i'), long, default_value = "5000")]
    ws_ping_interval: Option<u64>,

    /// Disable websocket ping
    #[arg(short('d'), long, action, conflicts_with = "ws_ping_interval")]
    disable_websocket_ping: bool,
}

impl Args {
    fn parse() -> Self {
        let mut args = <Args as Parser>::parse();
        if args.verbose {
            args.log_ws = true;
            args.http_log_level = 3;
        }
        if args.disable_websocket_ping {
            args.ws_ping_interval = None;
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
            let cancellation_token = CancellationToken::new();
            tokio::spawn({
                let cancellation_token = cancellation_token.clone();
                async move {
                    {
                        let _guard = cancellation_token.drop_guard();
                        let _ = ctrl_c().await;
                        info!("Got Ctrl-C, shutting down");
                    }

                    select! {
                        _ = ctrl_c() => {},
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
                    };
                    exit(1);
                }
            });

            let mut echo_server =
                EchoServer::new(args.port, args.http_log_level.into(), args.log_ws).await?;
            let ws_ping_interval = args.ws_ping_interval.map(std::time::Duration::from_millis);
            echo_server.set_ws_ping_interval(ws_ping_interval);

            info!("Starting echo server on {}", echo_server.local_addr());
            echo_server.run(cancellation_token).await
        })
        .map_err(Into::into)
}
