mod cli_handler;
mod multi_watcher;
mod watcher;
use cli_handler::Arguments;
use multi_watcher::{NamedWatcherStatus, PondWorkerError, ServiceWatcherPond};
use std::process::exit;

use crate::watcher::Status;

#[tokio::main]
async fn main() {
    let arguments = cli_handler::Arguments::parse();
    let pond = ServiceWatcherPond::new_from_stdin().unwrap();
    match arguments.interval {
        Some(interval) => loop {
            let start_time = std::time::Instant::now();
            run_once(&arguments, &pond).await.unwrap_or_else(|e| {
                eprintln!("Error: {:?}", e);
            });
            let end_time = std::time::Instant::now();
            let duration = end_time - start_time;
            tokio::time::sleep(interval - duration).await;
        },
        None => {
            run_once(&arguments, &pond).await.unwrap_or_else(|e| {
                eprintln!("Error: {:?}", e);
                exit(1);
            });
        }
    }
}

async fn run_once(arguments: &Arguments, pond: &ServiceWatcherPond) -> Result<(), PondWorkerError> {
    let output: Vec<NamedWatcherStatus> = pond.run(arguments.timeout).await?;
    print_output(output).await;
    Ok(())
}

async fn print_output(output: Vec<NamedWatcherStatus>) {
    for status in output {
        println!(
            "{} {}",
            status.name,
            match status.status {
                Status::Online(ping) => format!("ok {}", ping.as_millis()),
                Status::Offline(e) => match e {
                    watcher::ErrorType::Timeout => "timeout".to_string(),
                    watcher::ErrorType::WrongResponse => "fail".to_string(),
                    watcher::ErrorType::Unknown => "unknown".to_string(),
                },
            }
        );
    }
}
