use crate::ArpTable;
use ipnetwork::IpNetwork;
use pnet_datalink::{MacAddr, NetworkInterface};
use pnet_packet::ipv4::Ipv4Packet;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, RwLock};

pub(crate) struct Ipv4Handler {
    interfaces: Vec<NetworkInterface>,
    ipv4_addresses: Vec<Ipv4Addr>,
    arp_table: Arc<RwLock<ArpTable>>,
}

impl Ipv4Handler {
    pub(crate) fn new(interfaces: Vec<NetworkInterface>, arp_table: Arc<RwLock<ArpTable>>) -> Self {
        let ipv4_addresses = interfaces
            .iter()
            .map(|i| i.ips.iter().filter(|&ipn| ipn.is_ipv4()).clone())
            .flatten()
            .filter_map(|ipn| match ipn {
                IpNetwork::V4(ipv4n) => Some(ipv4n.ip()),
                IpNetwork::V6(_) => None,
            })
            .collect::<Vec<_>>();

        Ipv4Handler {
            interfaces,
            ipv4_addresses,
            arp_table,
        }
    }

    pub(crate) fn handle(&self, packet: Ipv4Packet) {
        if self.determine_if_ours(&packet) {
            println!("TODO");
            return;
        }

        {
            if let Some(mac_addr) = self
                .arp_table
                .read()
                .expect("read guard")
                .get(&packet.get_destination())
            {
                println!("mac_addr: {}", mac_addr);
            } else {
                // TODO: Send arp request
            }
        }
    }

    fn determine_if_ours(&self, packet: &Ipv4Packet) -> bool {
        let dest = packet.get_destination();
        self.ipv4_addresses.contains(&dest) || dest.is_broadcast()
    }
}
