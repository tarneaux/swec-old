/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
pub struct Args {
    #[clap(short, long, default_value = "/etc/swec/config.yml")]
    pub config: String,
}
