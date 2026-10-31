//! Minimal FFI bindings to the subset of the DPDK C API used by the BNG.
//!
//! The full DPDK API lives in C; some helpers used by the application
//! (burst APIs, memory pool accessors, mbuf accessors) are `static inline`
//! in the DPDK headers, so thin C shims (`c/bng_shim.{c,h}`) are used for those.

use std::os::raw::{c_char, c_int, c_uint, c_ushort};

pub const RTE_ETH_RSS_IPV4: u64 = 4; // RTE_ETH_RSS_IPV4 == RTE_BIT64(2)
pub const RTE_ETH_HASH_FUNCTION_TOEPLITZ: c_int = 1;

extern "C" {
    pub fn rte_eal_init(argc: c_int, argv: *mut *const c_char) -> c_int;
    pub fn rte_eth_dev_count_total() -> c_ushort;
    pub fn rte_eth_dev_get_port_by_name(name: *const c_char, port_id: *mut c_ushort) -> c_int;
    pub fn rte_eth_dev_configure(
        port_id: c_ushort,
        nb_rx_queue: c_ushort,
        nb_tx_queue: c_ushort,
        eth_conf: *const std::os::raw::c_void,
    ) -> c_int;
    pub fn rte_eth_rx_queue_setup(
        port_id: c_ushort,
        rx_queue_id: c_ushort,
        nb_rx_desc: c_ushort,
        socket_id: c_uint,
        rx_conf: *const std::os::raw::c_void,
        mb_pool: *mut std::os::raw::c_void,
    ) -> c_int;
    pub fn rte_eth_tx_queue_setup(
        port_id: c_ushort,
        tx_queue_id: c_ushort,
        nb_tx_desc: c_ushort,
        socket_id: c_uint,
        tx_conf: *const std::os::raw::c_void,
    ) -> c_int;
    pub fn rte_eth_dev_rx_queue_start(port_id: c_ushort, rx_queue_id: c_ushort) -> c_int;
    pub fn rte_eth_dev_tx_queue_start(port_id: c_ushort, tx_queue_id: c_ushort) -> c_int;
    pub fn rte_eth_dev_start(port_id: c_ushort) -> c_int;
    pub fn rte_eth_dev_close(port_id: c_ushort) -> c_int;
    pub fn rte_eth_dev_rss_hash_update(port_id: c_ushort, rss_conf: *mut std::os::raw::c_void) -> c_int;
    pub fn rte_vhost_driver_register(path: *const c_char, flags: c_uint) -> c_int;

    pub fn bng_shim_mempool_new(name: *const c_char, n: c_uint, elt_size: c_uint) -> *mut std::os::raw::c_void;
    pub fn bng_shim_mempool_free(mp: *mut std::os::raw::c_void);
    pub fn bng_shim_rx_burst(
        port_id: c_ushort,
        queue_id: c_ushort,
        rx_pkts: *mut *mut std::os::raw::c_void,
        nb_pkts: c_ushort,
    ) -> c_ushort;
    pub fn bng_shim_tx_burst(
        port_id: c_ushort,
        queue_id: c_ushort,
        tx_pkts: *mut *mut std::os::raw::c_void,
        nb_pkts: c_ushort,
    ) -> c_ushort;
    pub fn bng_shim_mbuf_data(m: *const std::os::raw::c_void) -> *mut std::os::raw::c_void;
}
