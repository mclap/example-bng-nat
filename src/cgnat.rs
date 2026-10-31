//! Minimal stateful NAT (NAT44-style) with an address pool.

use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FlowKey {
    src: u32,
    dst: u32,
    proto: u8,
    sport: u16,
    dport: u16,
}

struct Flow {
    orig: FlowKey,
    nat_src: u32,
    nat_port: u16,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub orig_src: u32,
    pub orig_dst: u32,
    pub orig_proto: u8,
    pub orig_sport: u16,
    pub orig_dport: u16,
    pub nat_src: u32,
    pub nat_port: u16,
}

pub struct Nat {
    public_pool: Vec<u32>,
    used: Vec<bool>,
    flows: HashMap<FlowKey, Flow>,
    /// Reverse translation index: (nat_src, nat_port) -> (orig_src, orig_sport).
    reverse: HashMap<(u32, u16), (u32, u16)>,
}

impl Nat {
    pub fn new(public_pool: &[u32]) -> Self {
        Self {
            public_pool: public_pool.to_vec(),
            used: vec![false; public_pool.len()],
            flows: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    fn alloc_addr(&mut self) -> u32 {
        for (i, u) in self.used.iter().enumerate() {
            if !*u {
                self.used[i] = true;
                return self.public_pool[i];
            }
        }
        // pool exhausted; wrap around and reuse (minimal PoC behaviour)
        0
    }

    /// Outbound translation: subscriber src -> public NAT address.
    /// Returns the (nat_src, nat_port) for the packet.
    pub fn translate_out(&mut self, src: u32, dst: u32, proto: u8, sport: u16, dport: u16) -> Option<(u32, u16)> {
        let key = FlowKey { src, dst, proto, sport, dport };
        if let Some(f) = self.flows.get(&key) {
            return Some((f.nat_src, f.nat_port));
        }
        let nat_src = self.alloc_addr();
        let nat_port = 1024 + (self.flows.len() as u16).max(1);
        self.flows.insert(key, Flow { orig: key, nat_src, nat_port });
        // populate reverse index for the return path
        self.reverse.insert((nat_src, nat_port), (src, sport));
        Some((nat_src, nat_port))
    }

    /// Inbound translation: look up the original subscriber tuple via the
    /// NAT identifiers of a returning packet.
    pub fn translate_in(&mut self, nat_src: u32, nat_port: u16) -> Option<(u32, u16)> {
        self.reverse.get(&(nat_src, nat_port)).copied()
    }

    pub fn sessions(&self) -> Vec<Session> {
        self.flows
            .values()
            .map(|f| Session {
                orig_src: f.orig.src,
                orig_dst: f.orig.dst,
                orig_proto: f.orig.proto,
                orig_sport: f.orig.sport,
                orig_dport: f.orig.dport,
                nat_src: f.nat_src,
                nat_port: f.nat_port,
            })
            .collect()
    }
}
