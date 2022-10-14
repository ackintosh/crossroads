use pnet_datalink::MacAddr;
use std::collections::HashMap;
use std::net::Ipv4Addr;

pub(crate) struct ArpTable {
    entries: HashMap<Ipv4Addr, MacAddr>,
}

impl ArpTable {
    pub(crate) fn new() -> Self {
        ArpTable {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, ipv4: &Ipv4Addr) -> Option<&MacAddr> {
        self.entries.get(ipv4)
    }
}
