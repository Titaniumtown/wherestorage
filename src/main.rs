
mod prelude;
mod cli;
mod speedtest_net;

use clap::Parser;
use cli::{Args, Subcmd};
use speedtest_net::{download_file, upload_file};

#[tokio::main]
async fn main() {
    
    match Args::parse().command {
        Subcmd::Down { url, depth } => download_file(url, depth).await,
        Subcmd::Up { file, retries } => upload_file(file, retries).await,
    }.unwrap_or_else(|e| {
        println!("{}",e.to_string())
    })
}


