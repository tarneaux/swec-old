mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use std::time::Duration;
use watcher::{OKWhen, ServiceWatcher};

#[tokio::main]
async fn main() {
    let mut pond = ServiceWatcherPond::new();
    let _ = pond.add_watcher(
        "github-tarneaux".to_string(),
        ServiceWatcher::new(
            "http://github.com/tarneaux/",
            OKWhen::InDom("supersplit".to_string()),
        ),
    );

    let _ = pond.add_watcher(
        "google-should-succeed".to_string(),
        ServiceWatcher::new("http://google.com/", OKWhen::InDom("google".to_string())),
    );

    let _ = pond.add_watcher(
        "google-should-fail".to_string(),
        ServiceWatcher::new(
            "http://google.com/",
            OKWhen::InDom("this will never be in a page".to_string()),
        ),
    );

    let _ = pond.add_watcher(
        "github-should-fail".to_string(),
        ServiceWatcher::new(
            "http://github.com/noonewilleverusethisname",
            OKWhen::Status(200),
        ),
    );

    let _ = pond.add_watcher(
        "This URL should never be reached".to_string(),
        ServiceWatcher::new(
            "http://arstarsttthngoogle.com/",
            OKWhen::InDom("google".to_string()),
        ),
    );

    let timeout = Duration::from_secs(5);

    loop {
        let statuses = pond.run(timeout).await.unwrap();
        println!("{:?}", statuses);
    }
}
