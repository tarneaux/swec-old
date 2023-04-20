use crate::watcher::ServiceWatcher;
use tokio::task::JoinSet;

pub struct ServiceWatcherPond {
    watchers: Vec<ServiceWatcher>,
}

impl ServiceWatcherPond {
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
        }
    }

    pub fn add_watcher(&mut self, watcher: ServiceWatcher) {
        self.watchers.push(watcher);
    }

    pub async fn run(&self) {
        let mut join_set = JoinSet::new();
        for watcher in self.watchers.iter() {
            let watcher = watcher.clone();
            join_set.spawn(async move {
                watcher.get_current_status().await;
            });
        }

        while let Some(res) = join_set.join_next().await {
            println!("res: {:?}", res);
        }
    }
}
