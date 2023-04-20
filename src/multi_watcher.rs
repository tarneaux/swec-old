use crate::watcher::{ServiceWatcher, Status};
use std::sync::{Arc, Mutex};
use tokio::task::JoinSet;

pub struct ServiceWatcherPond {
    watchers: Vec<ServiceWatcherWithStatus>,
}

pub struct ServiceWatcherWithStatus {
    watcher: ServiceWatcher,
    status: Arc<Mutex<Option<Status>>>,
}

impl ServiceWatcherPond {
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
        }
    }

    pub fn add_watcher(&mut self, watcher: ServiceWatcher) {
        self.watchers.push(ServiceWatcherWithStatus {
            watcher,
            status: Arc::new(Mutex::new(None)),
        });
    }

    pub async fn run(&self) {
        let mut join_set = JoinSet::new();
        for watcher_with_status in self.watchers.iter() {
            let watcher = watcher_with_status.watcher.clone();
            let status = watcher_with_status.status.clone();
            join_set.spawn(async move {
                let new_status = watcher.get_current_status().await;
                *status.lock().unwrap() = Some(new_status);
            });
        }

        while let Some(res) = join_set.join_next().await {
            println!("res: {:?}", res);
        }
    }

    pub async fn get_last_statuses(&self) {
        for watcher_with_status in self.watchers.iter() {
            let status = watcher_with_status.status.lock().unwrap();
            println!("status: {:?}", status);
        }
    }
}
