use crate::config::PortRole;

#[derive(Debug, Clone)]
pub struct Port {
    pub id: u16,
    pub name: String,
    pub role: PortRole,
    pub subnet: Option<String>,
    pub started: bool,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub net: u32,
    pub mask: u32,
    pub port_id: u16,
}

#[derive(Debug, Clone)]
pub struct Forwarder {
    ports: Vec<Port>,
    routes: Vec<Route>,
    default_route: Option<Route>,
}

fn mask_len(mask: u32) -> u32 {
    if mask == 0 {
        return 0;
    }
    let inv = !mask;
    32 - inv.trailing_zeros()
}

impl Forwarder {
    pub fn new(ports: Vec<Port>) -> Self {
        Self {
            ports,
            routes: Vec::new(),
            default_route: None,
        }
    }

    pub fn add_route(&mut self, net: u32, mask: u32, port_id: u16) {
        self.routes.push(Route { net, mask, port_id });
    }

    pub fn set_default_route(&mut self, net: u32, mask: u32, port_id: u16) {
        self.default_route = Some(Route { net, mask, port_id });
    }

    fn lookup(&self, dst_ip: u32) -> Option<u16> {
        let mut best: Option<&Route> = None;
        let mut best_len: u32 = 0;
        for r in &self.routes {
            if (dst_ip & r.mask) == (r.net & r.mask) {
                let len = mask_len(r.mask);
                if best.is_none() || len > best_len {
                    best = Some(r);
                    best_len = len;
                }
            }
        }
        if let Some(r) = best {
            return Some(r.port_id);
        }
        if let Some(d) = &self.default_route {
            return Some(d.port_id);
        }
        None
    }

    pub fn egress_for(&self, dst_ip: u32) -> Option<u16> {
        self.lookup(dst_ip)
    }
}
