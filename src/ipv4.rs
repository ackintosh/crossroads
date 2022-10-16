use crate::arp::{ArpEvent, ArpRequest};
use crate::ArpTable;
use ipnetwork::IpNetwork;
use pnet_datalink::{MacAddr, NetworkInterface};
use pnet_packet::ipv4::Ipv4Packet;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) const IPV4_ADDRESS_LENGTH: u8 = 4;

#[derive(Debug)]
pub(crate) enum Ipv4HandlerEvent {
    ReceivedPacket(Ipv4Packet<'static>),
}

struct Ipv4Handler {
    interfaces: Vec<NetworkInterface>,
    ipv4_addresses: Vec<Ipv4Addr>,
    arp_table: Arc<RwLock<ArpTable>>,
    receiver: UnboundedReceiver<Ipv4HandlerEvent>,
    sender_arp: UnboundedSender<ArpEvent>,
}

impl Ipv4Handler {
    fn new(
        interfaces: Vec<NetworkInterface>,
        arp_table: Arc<RwLock<ArpTable>>,
        receiver: UnboundedReceiver<Ipv4HandlerEvent>,
        sender_arp: UnboundedSender<ArpEvent>,
    ) -> Self {
        let ipv4_addresses = interfaces
            .iter()
            .flat_map(|i| i.ips.iter().filter(|&ipn| ipn.is_ipv4()))
            .filter_map(|ipn| match ipn {
                IpNetwork::V4(ipv4n) => Some(ipv4n.ip()),
                IpNetwork::V6(_) => None,
            })
            .collect::<Vec<_>>();

        Ipv4Handler {
            interfaces,
            ipv4_addresses,
            arp_table,
            receiver,
            sender_arp,
        }
    }

    fn handle_received_packet(&self, packet: Ipv4Packet) {
        if self.determine_if_ours(&packet) {
            println!("TODO");
            return;
        }

        if let Some(_mac_addr) = self
            .arp_table
            .read()
            .expect("read guard")
            .get(&packet.get_destination())
        {
            // TODO: Transfer the packet to an appropriate next hop.
        } else {
            // TODO: Fill the sender mac/ipv4 address with an actual one. Maybe routing table is
            // required to do that.
            if let Err(e) = self.sender_arp.send(ArpEvent::SendArpRequest(ArpRequest {
                sender_mac_address: MacAddr::zero(),      // TODO
                sender_ipv4_address: Ipv4Addr::BROADCAST, // TODO
                target_ipv4_address: packet.get_destination(),
            })) {
                println!("Failed to send ArpRequest to ArpHandler: {:?}", e);
            }
        }
    }

    fn determine_if_ours(&self, packet: &Ipv4Packet) -> bool {
        let dest = packet.get_destination();
        self.ipv4_addresses.contains(&dest) || dest.is_broadcast()
    }

    fn spawn(mut self) {
        println!("Ipv4Handler started");
        let fut = async move {
            loop {
                if let Some(event) = self.receiver.recv().await {
                    match event {
                        Ipv4HandlerEvent::ReceivedPacket(ipv4_packet) => {
                            self.handle_received_packet(ipv4_packet)
                        }
                    }
                }
            }
        };

        tokio::runtime::Handle::current().spawn(fut);
    }
}

pub(crate) async fn spawn_ipv4_handler(
    interfaces: Vec<NetworkInterface>,
    arp_table: Arc<RwLock<ArpTable>>,
    sender_arp: UnboundedSender<ArpEvent>,
) -> UnboundedSender<Ipv4HandlerEvent> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    Ipv4Handler::new(interfaces, arp_table, receiver, sender_arp).spawn();

    sender
}
