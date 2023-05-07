use clap::Parser;
use std::time::Duration;

#[derive(Parser)]
#[clap(author, version, about)]
struct ArgumentsRaw {
    /// The combined timeout for all watchers (run asynchroniously)
    #[arg(short, long, default_value = "10")]
    pub timeout: usize,
    /// The interval between the start of two watcher runs.
    #[arg(short, long)]
    pub interval: Option<usize>,
}

pub struct Arguments {
    pub timeout: Duration,
    pub interval: Option<Duration>,
}

impl Arguments {
    pub fn parse() -> Self {
        let arguments = ArgumentsRaw::parse();
        Self {
            timeout: Duration::from_secs(arguments.timeout as u64),
            interval: match arguments.interval {
                Some(i) => Some(Duration::from_secs(i as u64)),
                None => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let arguments = Arguments::parse();
        assert_eq!(arguments.timeout.as_secs(), 10);
        assert_eq!(arguments.interval, None);
    }
}
