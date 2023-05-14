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
    let pond = ServiceWatcherPond::new_from_stdin().unwrap_or_else(|e| {
        eprintln!("Error: {:?}", e);
        exit(1);
    });
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
    print_output(output);
    Ok(())
}

fn print_output(output: Vec<NamedWatcherStatus>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watcher::{OKWhen, ServiceWatcher};
    use cli_handler::Arguments;
    use multi_watcher::ServiceWatcherPond;
    use std::time::Duration;

    #[tokio::test]
    async fn test_run_once() {
        let arguments = Arguments {
            timeout: Duration::from_secs(5),
            interval: None,
        };
        let mut pond = ServiceWatcherPond::new();
        pond.add_watcher(
            "google".to_string(),
            ServiceWatcher::new("https://google.com", OKWhen::Status(200)),
        )
        .unwrap();
        run_once(&arguments, &pond).await.unwrap();
    }

    #[test]
    fn test_print_output() {
        let output = vec![
            NamedWatcherStatus {
                name: "google".to_string(),
                status: Status::Online(Duration::from_secs(0)),
            },
            NamedWatcherStatus {
                name: "google".to_string(),
                status: Status::Offline(watcher::ErrorType::Timeout),
            },
        ];
        print_output(output);
    }
}
