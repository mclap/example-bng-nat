mod cgnat;
mod config;
mod dpdk;
mod forward;
mod ip;
mod rest;
mod stats;

use std::ffi::CString;
use std::os::raw::{c_char, c_ushort};

use crate::config::{Config, PortRole};
use crate::cgnat::Nat;
use crate::forward::{Forwarder, Port};
use crate::stats::StatsState;

const ETH_P_IPV4: u16 = 0x0800;
const MAX_BURST: usize = 16;

fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("BNG_CONFIG").ok())
        .unwrap_or_else(|| "config.toml".to_string());
    let config = Config::load(&config_path)?;

    // --- EAL init ---
    let prog = CString::new("bng")?;
    let mut argv: [*const c_char; 1] = [prog.as_ptr()];
    let argv_ptr: *mut *const c_char = argv.as_mut_ptr() as *mut *const c_char;
    unsafe { dpdk::rte_eal_init(1, argv_ptr) };

    // --- vhost (QEMU virtio-net via vhost-user) ---
    let vhost_path = std::env::var("BNG_VHOST_PATH").unwrap_or_else(|_| "/tmp/bng_vhost.sock".to_string());
    let vhost_c = CString::new(vhost_path)?;
    unsafe { dpdk::rte_vhost_driver_register(vhost_c.as_ptr(), 0) };

    // --- open ethdevs and set up queues ---
    let mut ports: Vec<Port> = Vec::new();
    for cfg in &config.ports {
        let c_name = CString::new(cfg.name.as_str())?;
        let mut port_id: c_ushort = 0;
        let rc = unsafe { dpdk::rte_eth_dev_get_port_by_name(c_name.as_ptr(), &mut port_id) };
        if rc != 0 {
            anyhow::bail!("failed to open ethdev by name '{}'", cfg.name);
        }

        let mp_name = CString::new("bng_pool")?;
        let mp: *mut std::os::raw::c_void = unsafe { dpdk::bng_shim_mempool_new(mp_name.as_ptr(), 32, 2048) };
        if mp.is_null() {
            anyhow::bail!("failed to create mempool");
        }
        let mut eth_conf: [u64; 32] = [0; 32];
        let rc = unsafe { dpdk::rte_eth_dev_configure(port_id, 1, 1, eth_conf.as_mut_ptr() as *mut std::os::raw::c_void) };
        if rc != 0 {
            anyhow::bail!("failed to configure ethdev {}", cfg.name);
        }

        let mut rx_conf: [u64; 32] = [0; 32];
        let rc = unsafe { dpdk::rte_eth_rx_queue_setup(port_id, 0, 16, 0, rx_conf.as_mut_ptr() as *mut std::os::raw::c_void, mp) };
        if rc != 0 {
            anyhow::bail!("failed to setup rx queue on {}", cfg.name);
        }

        let mut tx_conf: [u64; 32] = [0; 32];
        let rc = unsafe { dpdk::rte_eth_tx_queue_setup(port_id, 0, 16, 0, tx_conf.as_mut_ptr() as *mut std::os::raw::c_void) };
        if rc != 0 {
            anyhow::bail!("failed to setup tx queue on {}", cfg.name);
        }

        let rc = unsafe { dpdk::rte_eth_dev_rx_queue_start(port_id, 0) };
        if rc != 0 {
            anyhow::bail!("failed to start rx queue on {}", cfg.name);
        }
        let rc = unsafe { dpdk::rte_eth_dev_tx_queue_start(port_id, 0) };
        if rc != 0 {
            anyhow::bail!("failed to start tx queue on {}", cfg.name);
        }
        let rc = unsafe { dpdk::rte_eth_dev_start(port_id) };
        if rc != 0 {
            anyhow::bail!("failed to start ethdev {}", cfg.name);
        }

        // RSS via DPDK (ipv4 hash, toeplitz)
        let mut rss_conf: [u64; 8] = [0; 8];
        unsafe {
            *(rss_conf.as_mut_ptr().add(2) as *mut u64) = dpdk::RTE_ETH_RSS_IPV4;
            *(rss_conf.as_mut_ptr().add(3) as *mut std::os::raw::c_int) = dpdk::RTE_ETH_HASH_FUNCTION_TOEPLITZ;
        }
        unsafe { dpdk::rte_eth_dev_rss_hash_update(port_id, rss_conf.as_mut_ptr() as *mut std::os::raw::c_void) };

        ports.push(Port {
            id: port_id,
            name: cfg.name.clone(),
            role: cfg.role,
            subnet: cfg.subnet.clone(),
            started: true,
        });
    }

    let mut fwd = Forwarder::new(ports.clone());
    for r in &config.routes {
        let (net, mask) = crate::ip::parse_cidr(&r.dst).unwrap_or((0, 0));
        let port_id = ports.iter().find(|p| p.name == r.port).map(|p| p.id).expect("route port not found");
        fwd.add_route(net, mask, port_id);
    }
    if let Some(uplink) = ports.iter().find(|p| p.role == PortRole::Uplink) {
        fwd.set_default_route(0, 0, uplink.id);
    }
    let nat = std::sync::Arc::new(std::sync::Mutex::new(Nat::new(&public_addrs(&config.cgnat))));
    let stats = StatsState::new();

    {
        let nat_srv = nat.clone();
        let stats_srv = stats.clone();
        std::thread::Builder::new()
            .name("bng-gnoi".to_string())
            .spawn(move || {
                let addr = std::env::var("BNG_GNOI_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
                if let Err(e) = rest::serve(&addr, nat_srv, stats_srv) {
                    eprintln!("gnoi server error: {}", e);
                }
            })
            .expect("failed to spawn gnoi thread");
    }

    loop {
        let mut nat_guard = nat.lock().unwrap();
        for pr in ports.iter() {
            let mut pkts: [*mut std::os::raw::c_void; MAX_BURST] = unsafe { std::mem::zeroed() };
            let pkts_ptr: *mut *mut std::os::raw::c_void = pkts.as_mut_ptr() as *mut *mut std::os::raw::c_void;
            let n: u16 = unsafe { dpdk::bng_shim_rx_burst(pr.id, 0, pkts_ptr, MAX_BURST as u16) };
            stats.count_rx(pr.id, n as u64);
            for i in 0..n as usize {
                let mbuf: *mut std::os::raw::c_void = pkts[i];
                handle_packet(pr, mbuf, &mut *nat_guard, &mut fwd, stats.as_ref());
            }
        }
    }
}

fn public_addrs(cgnat: &crate::config::Cgnat) -> Vec<u32> {
    cgnat
        .public_pool
        .iter()
        .filter_map(|s| crate::ip::ipv4_to_u32(s))
        .collect()
}

fn read_u16_be(p: *mut u8, off: usize) -> u16 {
    let b0 = unsafe { *p.add(off) };
    let b1 = unsafe { *p.add(off + 1) };
    u16::from_be_bytes([b0, b1])
}

fn write_u16_be(p: *mut u8, off: usize, v: u16) {
    unsafe {
        *p.add(off) = (v >> 8) as u8;
        *p.add(off + 1) = (v & 0xff) as u8;
    }
}

fn read_u32_be(p: *mut u8, off: usize) -> u32 {
    let b0 = unsafe { *p.add(off) };
    let b1 = unsafe { *p.add(off + 1) };
    let b2 = unsafe { *p.add(off + 2) };
    let b3 = unsafe { *p.add(off + 3) };
    u32::from_be_bytes([b0, b1, b2, b3])
}

fn write_u32_be(p: *mut u8, off: usize, v: u32) {
    unsafe {
        *p.add(off) = (v >> 24) as u8;
        *p.add(off + 1) = ((v >> 16) & 0xff) as u8;
        *p.add(off + 2) = ((v >> 8) & 0xff) as u8;
        *p.add(off + 3) = (v & 0xff) as u8;
    }
}

fn handle_packet(port: &Port, mbuf: *mut std::os::raw::c_void, nat: &mut Nat, fwd: &mut Forwarder, stats: &crate::stats::StatsState) {
    let data = unsafe { dpdk::bng_shim_mbuf_data(mbuf) } as *mut u8;
    let b12 = unsafe { *data.add(12) };
    let b13 = unsafe { *data.add(13) };
    let eth_type = u16::from_be_bytes([b12, b13]);
    let mut src_ip = 0u32;
    let mut dst_ip = 0u32;
    let mut proto = 0u8;
    let mut sport = 0u16;
    let mut dport = 0u16;
    if eth_type == ETH_P_IPV4 {
        proto = unsafe { *data.add(9) };
        src_ip = read_u32_be(data, 12);
        dst_ip = read_u32_be(data, 16);
        sport = read_u16_be(data, 20);
        dport = read_u16_be(data, 22);
    }

    let egress = if port.role == PortRole::Subscriber {
        // BNG: subscriber -> uplink, with egress NAT (src translation)
        if let Some((nat_src, nat_port)) = nat.translate_out(src_ip, dst_ip, proto, sport, dport) {
            write_u32_be(data, 12, nat_src);
            write_u16_be(data, 20, nat_port);
        }
        fwd.egress_for(dst_ip).expect("no egress route for subscriber packet")
    } else {
        // Router: uplink -> subscriber, with ingress NAT (dst translation)
        let mut effective_dst = dst_ip;
        if let Some((real_src, real_sport)) = nat.translate_in(dst_ip, dport) {
            write_u32_be(data, 16, real_src);
            write_u16_be(data, 22, real_sport);
            effective_dst = real_src;
        }
        fwd.egress_for(effective_dst).expect("no egress route for uplink packet")
    };

    let mut tx: [*mut std::os::raw::c_void; 1] = [mbuf];
    let tx_ptr: *mut *mut std::os::raw::c_void = tx.as_mut_ptr() as *mut *mut std::os::raw::c_void;
    stats.count_tx(egress, 1);
    unsafe { dpdk::bng_shim_tx_burst(egress, 0, tx_ptr, 1) };
}
