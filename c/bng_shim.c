#include "bng_shim.h"

void *bng_shim_mempool_new(const char *name, unsigned int n, unsigned int elt_size) {
    return rte_mempool_create(name, n, elt_size, 0, 0, NULL, NULL, NULL, NULL, 0, 0);
}

void bng_shim_mempool_free(void *mp) {
    rte_mempool_free(mp);
}

uint16_t bng_shim_rx_burst(uint16_t port_id, uint16_t queue_id, struct rte_mbuf **rx_pkts, uint16_t nb_pkts) {
    return rte_eth_rx_burst(port_id, queue_id, rx_pkts, nb_pkts);
}

uint16_t bng_shim_tx_burst(uint16_t port_id, uint16_t queue_id, struct rte_mbuf **tx_pkts, uint16_t nb_pkts) {
    return rte_eth_tx_burst(port_id, queue_id, tx_pkts, nb_pkts);
}

void *bng_shim_mbuf_data(const struct rte_mbuf *m) {
    return (void *)rte_mbuf_data_iova_default(m);
}
