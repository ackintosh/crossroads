use std::net::{IpAddr, Ipv4Addr};
use ipnetwork::IpNetwork;
use pnet_datalink::NetworkInterface;
use pnet_packet::ipv4::Ipv4Packet;

pub(crate) struct Ipv4Handler {
    interfaces: Vec<NetworkInterface>,
    ipv4_addresses: Vec<Ipv4Addr>,
}

impl Ipv4Handler {
    pub(crate) fn new(interfaces: Vec<NetworkInterface>) -> Self {
        let ipv4_addresses = interfaces.iter()
            .map(|i| i.ips.iter().filter(|&ipn| ipn.is_ipv4()).clone())
            .flatten()
            .filter_map(|ipn| {
                match ipn {
                    IpNetwork::V4(ipv4n) => Some(ipv4n.ip()),
                    IpNetwork::V6(_) => None,
                }
            })
            .collect::<Vec<_>>();

        Ipv4Handler {
            interfaces,
            ipv4_addresses,
        }
    }

    pub(crate) fn handle(&self, packet: Ipv4Packet) {
        if self.determine_if_ours(&packet) {
            println!("TODO");
            return;
        }


    }

    fn determine_if_ours(&self, packet: &Ipv4Packet) -> bool {
        let dest = packet.get_destination();
        self.ipv4_addresses.contains(&dest) || dest.is_broadcast()
    }
}
