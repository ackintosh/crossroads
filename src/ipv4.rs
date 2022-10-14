use crate::ArpTable;
use ipnetwork::IpNetwork;
use pnet_datalink::{MacAddr, NetworkInterface};
use pnet_packet::ipv4::Ipv4Packet;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub(crate) enum Ipv4HandlerEvent {
    ReceivedPacket(Ipv4Packet<'static>),
}

struct Ipv4Handler {
    interfaces: Vec<NetworkInterface>,
    ipv4_addresses: Vec<Ipv4Addr>,
    arp_table: Arc<RwLock<ArpTable>>,
    receiver: UnboundedReceiver<Ipv4HandlerEvent>,
}

impl Ipv4Handler {
    fn new(
        interfaces: Vec<NetworkInterface>,
        arp_table: Arc<RwLock<ArpTable>>,
        receiver: UnboundedReceiver<Ipv4HandlerEvent>,
    ) -> Self {
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
            receiver,
        }
    }

    fn handle(&self, packet: Ipv4Packet) {
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

    fn spawn(mut self) {
        println!("Ipv4Handler started");
        let fut = async move {
            loop {
                if let Some(event) = self.receiver.recv().await {
                    match event {
                        Ipv4HandlerEvent::ReceivedPacket(ipv4_packet) => self.handle(ipv4_packet),
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
) -> UnboundedSender<Ipv4HandlerEvent> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    Ipv4Handler::new(interfaces, arp_table, receiver).spawn();

    sender
}
