/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use clap::Parser;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::BufWriter;

mod api;
mod argument_parser;
mod config;
mod handlers;
mod watchers;

use api::api_server;
use argument_parser::Args;
use config::Config;
use handlers::histfile::{read_histories_from_file, restore_histories_to_pond, HistfileHandler};
use watchers::WatcherPond;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config = Config::read(&args.config).unwrap_or_else(|e| {
        eprintln!("Error while reading config file: {}", e);
        std::process::exit(1);
    });

    let histories = read_histories_from_file("./histfile");

    let file = match File::create("./histfile").await {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error while opening histfile: {}", e);
            std::process::exit(1);
        }
    };

    let pond = WatcherPond::new(
        config.watchers,
        config.histsize,
        Duration::from_secs(config.interval),
        vec![Box::new(HistfileHandler::new(BufWriter::new(file)))],
    );

    let mut pond = match histories {
        Ok(histories) => restore_histories_to_pond(histories, pond).await,
        Err(e) => {
            eprintln!("Error while reading histfile: {}", e);
            pond
        }
    };

    let status_histories = pond.status_histories.clone();
    let watchers = pond.watchers.clone();

    tokio::spawn(async move {
        api_server(status_histories, watchers).await;
    });

    pond.watch().await;
}
