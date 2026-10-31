# Minimal BNG + router on Rust + DPDK, with minimal CGNAT.

Minimal user-space BNG/router:

- multiple Ethernet ports, each with a role (`subscriber` | `uplink`) taken from
  the config file;
- static routes for forwarding (longest-prefix lookup with default route);
- minimal stateful NAT (NAT44-style) with configurable public address pool;
- RSS configured through the DPDK API (`rte_eth_dev_rss_hash_update`);
- virtio NIC support via QEMU (vhost-user).

## Build

```sh
cargo build
```

Requires DPDK (with dev headers/libs) installed, e.g. from DPDK 25.x.

## Run

```sh
sudo ./target/debug/bng /path/to/config.toml
```

Config (`config.toml`) example:

```toml
[[ports]]
name = "subscriber0"
role = "subscriber"
subnet = "10.0.0.0/24"

[[ports]]
name = "uplink0"
role = "uplink"

[cgnat]
enabled = true
listen_subnet = "10.0.0.0/24"
public_pool = ["203.0.113.10", "203.0.113.11", "203.0.113.12"]

[[routes]]
dst = "0.0.0.0/0"
port = "uplink0"

[[routes]]
dst = "10.0.0.0/24"
port = "subscriber0"
```

## Static routes

Static routes are installed into the forwarding table with longest-prefix
lookup and a default route fallback.

## REST: stats & sessions, with OpenAPI

A minimal REST server exposing stats & sessions, with OpenAPI document:

- `GET /openapi.json` — OpenAPI document;
- `GET /stats` — JSON traffic statistics (global + per-port counters);
- `GET /sessions` — JSON list of NAT sessions.

Example:

```sh
curl http://127.0.0.1:8080/openapi.json
curl http://127.0.0.1:8080/stats
curl http://127.0.0.1:8080/sessions
```

## QEMU + virtio (no HW available)

Run QEMU with a vhost-user backend and let DPDK use the VHOST driver:

```sh
qemu-system-x86_64 \
  -enable-kvm -cpu host -m 2048 -smp 2 \
  -netdev vhost-user,id=n0,sock=/tmp/bng_vhost.sock \
  -device virtio-net-pci,netdev=n0 ...
```

The DPDK app registers the vhost-user socket via `BNG_VHOST_PATH` (default
`/tmp/bng_vhost.sock`) using `rte_vhost_driver_register()`.

## Notes

- No VLAN support in this minimal version.
- RSS is set through DPDK for IPv4 (Toeplitz) on every started port.
- Forwarding uses static routes: dst network -> egress port, with default
  route fallback.
