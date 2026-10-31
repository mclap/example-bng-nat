#ifndef BNG_SHIM_H
#define BNG_SHIM_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#include <rte_mbuf.h>
#include <rte_mempool.h>
#include <rte_ethdev.h>

void *bng_shim_mempool_new(const char *name, unsigned int n, unsigned int elt_size);
void bng_shim_mempool_free(void *mp);

uint16_t bng_shim_rx_burst(uint16_t port_id, uint16_t queue_id, struct rte_mbuf **rx_pkts, uint16_t nb_pkts);
uint16_t bng_shim_tx_burst(uint16_t port_id, uint16_t queue_id, struct rte_mbuf **tx_pkts, uint16_t nb_pkts);

void *bng_shim_mbuf_data(const struct rte_mbuf *m);

#ifdef __cplusplus
}
#endif

#endif
