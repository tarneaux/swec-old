mod cli_handler;
mod multi_watcher;
mod watcher;
use cli_handler::Arguments;
use multi_watcher::ServiceWatcherPond;

#[tokio::main]
async fn main() {
    let arguments = cli_handler::Arguments::parse();
    let pond = ServiceWatcherPond::new_from_stdin().unwrap();
    match arguments.interval {
        Some(interval) => loop {
            let start_time = std::time::Instant::now();
            run_once(&arguments, &pond).await;
            let end_time = std::time::Instant::now();
            let duration = end_time - start_time;
            tokio::time::sleep(interval - duration).await;
        },
        None => {
            run_once(&arguments, &pond).await;
        }
    }
}

async fn run_once(arguments: &Arguments, pond: &ServiceWatcherPond) {
    let output = pond.run(arguments.timeout).await;
    print_output(output).await;
}

async fn print_output(output: Vec<(String, watcher::Status)>) {
    for (name, status) in output {
        println!("{}: {:?}", name, status);
    }
}
