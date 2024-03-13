use clap::{Parser, Subcommand};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use swec_client::client::{Api, ReadApi, ReadOnly, ReadWrite, WriteApi};
use swec_core::{Spec, Status};
use tokio::main;
use tokio::sync::mpsc;

#[main]
async fn main() {
    let opts: Opts = Opts::parse();
    match opts.subcmd {
        Command::Get { checker, what } => {
            let base_url = opts
                .base_url
                .unwrap_or_else(|| "http://localhost:8080/api/v1".to_string());

            let client = ReadOnly::new(base_url).expect("Failed to create API client");
            client.get_info().await.expect("Failed to get API info");
            let checker = checker.unwrap();
            match what {
                GetWhat::Spec => {
                    println!("{}", client.get_checker_spec(&checker).await.unwrap());
                }
                GetWhat::Statuses => {
                    println!("{:?}", client.get_checker_statuses(&checker).await.unwrap());
                }
                GetWhat::Watch => {
                    let (tx, mut rx) = mpsc::channel(32);
                    println!("{:?}", client.watch_checker(&checker, tx).await);
                    while let Some(status) = rx.recv().await {
                        println!("{status}");
                    }
                }
            }
        }
        v => {
            let base_url = opts
                .base_url
                .unwrap_or_else(|| "http://localhost:8081/api/v1".to_string());
            let client = ReadWrite::new(base_url).expect("Failed to create API client");
            let api_info = client.get_info().await.unwrap_or_else(|e| {
                eprintln!("Failed to get API info: {e}");
                std::process::exit(1);
            });
            if !api_info.writable {
                eprintln!(
                    "This API endpoint, while being a valid SWEC API, is not writable. Exiting."
                );
                std::process::exit(1);
            }
            match v {
                Command::Post { checker, what } => match what {
                    PostWhat::Spec { spec } => {
                        println!("{:?}", client.post_checker_spec(&checker, spec).await);
                    }
                    PostWhat::Status { status } => {
                        println!("{:?}", client.post_checker_status(&checker, status).await);
                    }
                },
                Command::Delete { checker } => {
                    println!("{:?}", client.delete_checker(&checker).await);
                }
                Command::Put { checker, spec } => {
                    println!("{:?}", client.put_checker_spec(&checker, spec).await);
                }
                Command::Get { .. } => unreachable!(), // already handled above
            }
        }
    }
}

#[derive(Parser, Debug)]
#[clap(version, about, author)]
struct Opts {
    /// The base URL of the API. If not specified, we will use either https://localhost:8080/api/v1
    /// (if reading) or https://localhost:8081/api/v1 (if writing)
    #[clap(long)]
    base_url: Option<String>,

    #[clap(subcommand)]
    subcmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Get data from the server
    Get {
        /// What to get
        what: GetWhat,

        /// The checker to get data for
        checker: Option<String>,
    },
    /// Post a checker spec or status to the server
    Post {
        /// The checker to post to
        checker: String,

        /// The checker spec or status to post
        #[clap(subcommand)]
        what: PostWhat,
    },
    /// Put a checker spec to the server
    Put {
        /// The checker to put to
        checker: String,

        /// The checker spec to put
        spec: Spec,
    },
    /// Delete a checker from the server
    Delete {
        /// The checker to delete
        checker: String,
    },
}

#[derive(Parser, Debug, Clone)]
enum GetWhat {
    Spec,
    Statuses,
    Watch,
}

impl FromStr for GetWhat {
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

#[derive(Subcommand, Debug)]
enum PostWhat {
    Spec {
        /// The spec to post
        spec: Spec,
    },
    Status {
        /// The status to post
        status: Status,
    },
}

#[derive(Debug, Clone)]
struct UnknownValueError(String);

impl Display for UnknownValueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown value: {}", self.0)
    }
}

impl Error for UnknownValueError {}
