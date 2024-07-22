
mod prelude;
mod cli;
mod utils;
mod speedtest_net;

use prelude::*;
use clap::Parser;
use cli::{Args, Subcmd};
use speedtest_net::{download_file, upload_file};

#[tokio::main]
async fn main() -> Result<()> {
    match Args::parse().command {
        Subcmd::Down { url, depth } => download_file(url, depth).await,
        Subcmd::Up { file, retries } => upload_file(file, retries).await,
    }
}


