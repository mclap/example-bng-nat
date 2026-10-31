//! Minimal IPv4 helpers.

/// Parse a dotted-quad IPv4 string into a big-endian u32.
pub fn ipv4_to_u32(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut v: u32 = 0;
    for p in parts {
        let n: u32 = p.parse().ok()?;
        if n > 255 {
            return None;
        }
        v = (v << 8) | n;
    }
    Some(v)
}

/// Parse an IPv4 CIDR into (network, mask).
pub fn parse_cidr(s: &str) -> Option<(u32, u32)> {
    let (ip_str, bits_str) = s.split_once('/')?;
    let ip = ipv4_to_u32(ip_str)?;
    let bits: u32 = bits_str.parse().ok()?;
    if bits > 32 {
        return None;
    }
    let mask = if bits == 0 { 0 } else { !0u32 << (32 - bits) };
    Some((ip & mask, mask))
}

/// Check whether `ip` is inside `net`/`mask`.
pub fn in_cidr(ip: u32, net: u32, mask: u32) -> bool {
    ip & mask == net & mask
}
