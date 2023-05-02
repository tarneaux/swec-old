use crate::multi_watcher::ServiceWatcherPond;
use crate::watcher::{ServiceWatcher, Status};
use std::time::Duration;

pub struct StatusHistoryPond {
    histories: Vec<StatusHistory>,
    service_watcher_pond: ServiceWatcherPond,
    max_statuses: usize,
}

impl StatusHistoryPond {
    pub fn new(max_statuses: usize) -> Self {
        Self {
            histories: Vec::new(),
            service_watcher_pond: ServiceWatcherPond::new(),
            max_statuses,
        }
    }

    pub fn add_watcher(&mut self, watcher: ServiceWatcher) {
        self.service_watcher_pond.add_watcher(watcher);
        self.histories.push(StatusHistory::new(self.max_statuses));
    }

    pub async fn run(&mut self, timeout: Duration) {
        self.service_watcher_pond.run(timeout).await;
        let statuses = self.service_watcher_pond.get_last_statuses().await;
        for (i, status) in statuses.iter().enumerate() {
            self.histories[i].add_status(*status);
        }
    }

    pub fn get_statuses(&self, index: usize) -> Vec<Option<Status>> {
        self.histories[index].statuses.clone()
    }

    pub fn get_last_statuses(&self) -> Vec<Option<Status>> {
        let mut return_value = Vec::new();
        for history in self.histories.iter() {
            return_value.push(history.get_last_status());
        }
        return_value
    }
}

pub struct StatusHistory {
    statuses: Vec<Option<Status>>,
    max_statuses: usize,
}

impl StatusHistory {
    pub fn new(max_statuses: usize) -> Self {
        Self {
            statuses: Vec::new(),
            max_statuses,
        }
    }

    pub fn add_status(&mut self, status: Option<Status>) {
        self.statuses.push(status);
        if self.statuses.len() > self.max_statuses {
            self.statuses.remove(0);
        }
    }

    pub fn get_last_status(&self) -> Option<Status> {
        match self.statuses.last() {
            Some(status) => match status {
                Some(status) => Some(*status),
                None => None,
            },
            None => None,
        }
    }
}
