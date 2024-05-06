use std::{collections::VecDeque, ops::Range, sync::Arc};

use ahash::AHashSet;
use parking_lot::{Mutex, RwLock};
use tracing_log::log::error;

use crate::{error::AppResult, repositories::ChampionshipRepository};

pub const PORTS_RANGE: Range<i32> = 27700..27800;

#[derive(Clone)]
pub struct MachinePorts {
    ports: Arc<Mutex<VecDeque<i32>>>,
    used_ports: Arc<RwLock<AHashSet<i32>>>,
}

impl MachinePorts {
    pub async fn new(championship_repo: &ChampionshipRepository) -> AppResult<Self> {
        let ports_used = championship_repo.ports_in_use().await?;
        let estimated_len = PORTS_RANGE.len() - ports_used.len();
        let mut ports = VecDeque::with_capacity(estimated_len);

        for port in PORTS_RANGE {
            if !ports_used.contains(&port) {
                ports.push_back(port);
            }
        }

        let ports = Arc::new(Mutex::new(ports));

        Ok(MachinePorts {
            ports,
            used_ports: Arc::new(RwLock::new(ports_used)),
        })
    }

    pub fn next(&self) -> Option<i32> {
        let mut ports = self.ports.lock();

        match ports.pop_front() {
            None => None,
            Some(port) => {
                let mut used_ports = self.used_ports.write();
                used_ports.insert(port);
                Some(port)
            }
        }
    }

    pub fn return_port(&self, port: i32) {
        if !PORTS_RANGE.contains(&port) {
            error!("Port {} is not in the range", port);
            return;
        }

        let mut ports = self.ports.lock();

        if !ports.contains(&port) {
            ports.push_back(port);
        }
    }
}
