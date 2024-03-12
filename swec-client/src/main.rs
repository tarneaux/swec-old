use clap::{Parser, Subcommand};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use swec_client::client::{Api, ReadApi, ReadOnly};
use tokio::main;
use tokio::sync::mpsc;

#[main]
async fn main() {
    let opts: Opts = Opts::parse();
    let client = ReadOnly::new(opts.base_url).unwrap();
    println!("{:?}", client.get_info().await.unwrap());
    match opts.subcmd {
        Command::Get { watcher, what } => {
            let watcher = watcher.unwrap();
            match what {
                What::Spec => {
                    println!("{}", client.get_watcher_spec(&watcher).await.unwrap());
                }
                What::Statuses => {
                    println!("{:?}", client.get_watcher_statuses(&watcher).await.unwrap());
                }
                What::Watch => {
                    let (tx, mut rx) = mpsc::channel(32);
                    println!("{:?}", client.watch_watcher(&watcher, tx).await);
                    while let Some(status) = rx.recv().await {
                        println!("{status:?}");
                    }
                }
            }
        }
    }
}

#[derive(Parser, Debug)]
#[clap(version, about, author)]
struct Opts {
    /// The base URL of the API
    #[clap(long, default_value = "http://localhost:8080/api/v1")]
    base_url: String,

    #[clap(subcommand)]
    subcmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Get data from the server
    Get {
        /// What to get
        what: What,
        /// The watcher to get data for
        watcher: Option<String>,
    }, // TODO: post, put, delete
}

#[derive(Parser, Debug, Clone)]
enum What {
    Spec,
    Statuses,
    Watch,
}

impl FromStr for What {
    type Err = UnknownValueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "spec" => Ok(Self::Spec),
            "statuses" => Ok(Self::Statuses),
            "watch" => Ok(Self::Watch),
            _ => Err(UnknownValueError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
struct UnknownValueError(String);

impl Display for UnknownValueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown value: {}", self.0)
    }
}

impl Error for UnknownValueError {}
