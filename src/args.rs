use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
    #[arg(short, long)]
    pub root: Option<PathBuf>,
}
