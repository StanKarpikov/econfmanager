use arguments::Args;
use clap::Parser;
use jsonrpc_lib::{build_server, utils::setup_logging};
use log::info;

pub mod arguments;

const SERVE_STATIC_FILES: bool = true;
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    setup_logging();
    
    let args = Args::parse();

    info!("Starting server with configuration: {}", args.config);

    build_server!(args.config, 
                  SERVE_STATIC_FILES, 
                  warp::path("health").map(|| "OK"),
                  warp::path("version").map(|| VERSION.unwrap_or("unknown")));
}
