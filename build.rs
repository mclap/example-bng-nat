fn main() {
    cc::Build::new()
        .include("/usr/include")
        .file("c/bng_shim.c")
        .compile("bng_shim");

    pkg_config::probe_library("libdpdk")
        .unwrap_or_else(|e| panic!("failed to probe dpdk via pkg-config: {}", e));
}
