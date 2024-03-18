use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    pub config_path: PathBuf,
}


pub fn cli() -> Args {
    Args::parse()
}
