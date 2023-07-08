use clap::Parser;
use std::fmt::Debug;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
pub struct Args {
    #[clap(short, long, default_value = "/etc/swec/config.yml")]
    pub config: String,
}
