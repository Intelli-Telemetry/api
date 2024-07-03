use std::sync::Arc;

use parking_lot::Mutex;
use tracing_log::log::error;

use crate::{
    config::constants::PORTS_RANGE, error::AppResult, repositories::ChampionshipRepository,
};

#[derive(Clone)]
pub struct MachinePorts {
    ports: Arc<Mutex<Vec<i32>>>,
}

impl MachinePorts {
    pub async fn new(championship_repo: &ChampionshipRepository) -> AppResult<Self> {
        let ports_used = championship_repo.ports_in_use().await?;
        let estimated_len = PORTS_RANGE.len() - ports_used.len();
        let mut ports = Vec::with_capacity(estimated_len);

        for port in PORTS_RANGE {
            if !ports_used.contains(&port) {
                ports.push(port);
            }
        }

        let ports = Arc::new(Mutex::new(ports));

        Ok(MachinePorts { ports })
    }

    pub fn next(&self) -> Option<i32> {
        let mut ports = self.ports.lock();
        ports.pop()
    }

    pub fn return_port(&self, port: i32) {
        if !PORTS_RANGE.contains(&port) {
            error!("Port {} is not in the range", port);
            return;
        }

        let mut ports = self.ports.lock();

        if !ports.contains(&port) {
            ports.push(port);
        }
    }
}
