use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct StatsState {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub port_counters: Mutex<HashMap<u16, (u64, u64)>>,
}

impl StatsState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn count_rx(&self, port: u16, n: u64) {
        self.port_counters.lock().unwrap().entry(port).or_insert((0, 0)).0 += n;
    }

    pub fn count_tx(&self, port: u16, n: u64) {
        self.port_counters.lock().unwrap().entry(port).or_insert((0, 0)).1 += n;
    }

    pub fn snapshot(&self) -> (u64, u64, Vec<(u16, u64, u64)>) {
        let counters = self.port_counters.lock().unwrap();
        let mut ports: Vec<(u16, u64, u64)> = counters.iter().map(|(k, (r, t))| (*k, *r, *t)).collect();
        ports.sort_by_key(|(k, _, _)| *k);
        (self.rx_packets, self.tx_packets, ports)
    }
}
