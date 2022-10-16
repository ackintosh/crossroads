use crate::ethernet::{ETHERNET_ADDRESS_LENGTH, ETHERNET_TYPE_IP};
use crate::ipv4::IPV4_ADDRESS_LENGTH;
use ipnetwork::IpNetwork;
use pnet_datalink::{MacAddr, NetworkInterface};
use pnet_packet::arp::{Arp, ArpHardwareType, ArpOperation, ArpPacket};
use pnet_packet::ethernet::EtherType;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedReceiver;

const ARP_HARDWARE_TYPE_ETHERNET: u16 = 0x0001;

const ARP_OPERATION_CODE_REQUEST: u16 = 0x0001;
const ARP_OPERATION_CODE_REPLY: u16 = 0x0002;

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

    pub(crate) fn put(&mut self, ipv4: Ipv4Addr, mac: MacAddr) {
        if let Some(old) = self.entries.insert(ipv4, mac) {
            println!(
                "Replaced ARP table. ipv4: {}, old_mac: {}, new_mac: {}",
                ipv4, mac, old
            );
        }
    }
}

#[derive(Debug)]
pub(crate) enum ArpHandlerEvent {
    /// Received an ARP packet.
    ReceivedPacket(ArpPacket<'static>),
    /// An event let ArpHandler to send ARP request.
    SendArpRequest(ArpRequest),
}

#[derive(Debug)]
pub(crate) struct ArpRequest {
    pub(crate) sender_mac_address: MacAddr,
    pub(crate) sender_ipv4_address: Ipv4Addr,
    pub(crate) target_ipv4_address: Ipv4Addr,
}

struct ArpHandler {
    arp_table: Arc<RwLock<ArpTable>>,
    interfaces: HashMap<Ipv4Addr, NetworkInterface>,
    receiver: UnboundedReceiver<ArpHandlerEvent>,
}

impl ArpHandler {
    fn spawn(mut self) {
        let fut = async move {
            loop {
                if let Some(event) = self.receiver.recv().await {
                    match event {
                        ArpHandlerEvent::ReceivedPacket(arp_packet) => {
                            match arp_packet.get_operation().0 {
                                ARP_OPERATION_CODE_REQUEST => {
                                    self.handle_request_packet(arp_packet)
                                }
                                // TODO: Handle ARP response operation
                                other => println!("Unsupported ARP operation code: {}", other),
                            }
                        }
                        ArpHandlerEvent::SendArpRequest(request) => {
                            // https://docs.rs/pnet/latest/pnet/packet/arp/struct.Arp.html
                            let _arp = self.construct_request(request);
                            // TODO: Send the arp request via ethernet handler.
                        }
                    }
                }
            }
        };

        tokio::runtime::Handle::current().spawn(fut);
    }

    fn handle_request_packet(&self, packet: ArpPacket<'static>) {
        // Update ARP table with the source mac/ipv4 address.
        self.arp_table
            .write()
            .expect("write guard")
            .put(packet.get_sender_proto_addr(), packet.get_sender_hw_addr());

        // Determine if the packet is ours.
        if let Some(interface) = self.interfaces.get(&packet.get_target_proto_addr()) {
            let _reply = self.construct_reply(
                interface.mac.expect("should have mac address"),
                packet.get_target_proto_addr(),
                packet.get_sender_hw_addr(),
                packet.get_sender_proto_addr(),
            );
            // TODO: Send the arp reply via ethernet handler.
        }
    }

    fn construct_request(&self, request: ArpRequest) -> Arp {
        Arp {
            hardware_type: ArpHardwareType(ARP_HARDWARE_TYPE_ETHERNET),
            protocol_type: EtherType(ETHERNET_TYPE_IP),
            hw_addr_len: ETHERNET_ADDRESS_LENGTH,
            proto_addr_len: IPV4_ADDRESS_LENGTH,
            operation: ArpOperation(ARP_OPERATION_CODE_REQUEST),
            sender_hw_addr: request.sender_mac_address,
            sender_proto_addr: request.sender_ipv4_address,
            target_hw_addr: MacAddr::broadcast(),
            target_proto_addr: request.target_ipv4_address,
            payload: vec![],
        }
    }

    fn construct_reply(
        &self,
        sender_mac_address: MacAddr,
        sender_ipv4_address: Ipv4Addr,
        target_mac_address: MacAddr,
        target_ipv4_address: Ipv4Addr,
    ) -> Arp {
        Arp {
            hardware_type: ArpHardwareType(ARP_HARDWARE_TYPE_ETHERNET),
            protocol_type: EtherType(ETHERNET_TYPE_IP),
            hw_addr_len: ETHERNET_ADDRESS_LENGTH,
            proto_addr_len: IPV4_ADDRESS_LENGTH,
            operation: ArpOperation(ARP_OPERATION_CODE_REPLY),
            sender_hw_addr: sender_mac_address,
            sender_proto_addr: sender_ipv4_address,
            target_hw_addr: target_mac_address,
            target_proto_addr: target_ipv4_address,
            payload: vec![],
        }
    }
}

pub(crate) async fn spawn_arp_handler(
    interfaces: &Vec<NetworkInterface>,
    arp_table: Arc<RwLock<ArpTable>>,
    receiver: UnboundedReceiver<ArpHandlerEvent>,
) {
    let mut interface_map = HashMap::new();
    for i in interfaces {
        i.ips
            .iter()
            .filter_map(|ipn| match ipn {
                IpNetwork::V4(ipv4n) => Some(ipv4n.ip()),
                IpNetwork::V6(_) => None,
            })
            .for_each(|ipv4| {
                interface_map.insert(ipv4, i.clone());
            });
    }

    ArpHandler {
        arp_table,
        receiver,
        interfaces: interface_map,
    }
    .spawn();
}
