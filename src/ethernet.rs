use crate::arp::ArpHandlerEvent;
use crate::ipv4::Ipv4HandlerEvent;
use async_stream::stream;
use futures_util::{pin_mut, StreamExt};
use pnet_datalink::{Config, DataLinkReceiver, NetworkInterface};
use pnet_packet::arp::ArpPacket;
use pnet_packet::ipv4::Ipv4Packet;
use pnet_packet::Packet;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) const ETHERNET_TYPE_IP: u16 = 0x0800;
pub(crate) const ETHERNET_TYPE_ARP: u16 = 0x0806;

pub(crate) const ETHERNET_ADDRESS_LENGTH: u8 = 6;

pub(crate) enum EthernetHandlerEvent {}

pub(crate) async fn spawn_ethernet_handler(
    interfaces: &Vec<NetworkInterface>,
    receiver: UnboundedReceiver<EthernetHandlerEvent>,
    sender_arp: UnboundedSender<ArpHandlerEvent>,
    sender_ipv4: UnboundedSender<Ipv4HandlerEvent>,
) {
    EthernetHandler {
        interfaces: interfaces.clone(),
        receiver,
        sender_arp,
        sender_ipv4,
    }
    .spawn();
}

struct EthernetHandler {
    interfaces: Vec<NetworkInterface>,
    receiver: UnboundedReceiver<EthernetHandlerEvent>,
    sender_arp: UnboundedSender<ArpHandlerEvent>,
    sender_ipv4: UnboundedSender<Ipv4HandlerEvent>,
}

struct Receiver {
    /// The interface index (operating system specific).
    interface_index: u32,
    /// Structure for receiving packets at the data link layer.
    rx: Box<dyn DataLinkReceiver>,
}

#[derive(Debug)]
struct ReceivedPacket {
    /// The interface index (operating system specific).
    interface_index: u32,
    ethernet_packet: pnet_packet::ethernet::EthernetPacket<'static>,
}

impl EthernetHandler {
    fn spawn(mut self) {
        let config = Config {
            // Specifying read timeout to be `0` in order to let the receiver have non-blocking behavior.
            // https://github.com/libpnet/libpnet/issues/343#issuecomment-406866437
            read_timeout: Some(Duration::from_secs(0)),
            ..Default::default()
        };

        let mut receivers = self
            .interfaces
            .iter()
            .map(|i| {
                let (_tx, rx) = match pnet_datalink::channel(i, config) {
                    Ok(pnet_datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
                    Ok(_) => panic!("Unhandled channel type"),
                    Err(e) => panic!(
                        "An error occurred when creating the datalink channel: {}",
                        e
                    ),
                };

                Receiver {
                    interface_index: i.index,
                    rx,
                }
            })
            .collect::<Vec<_>>();

        let fut = async move {
            let stream = stream! {
                loop {
                    for r in receivers.iter_mut() {
                        match r.rx.next() {
                            Ok(packet) => {
                                // pnet::packet::ethernet::EthernetPacket
                                // https://docs.rs/pnet/latest/pnet/packet/ethernet/struct.EthernetPacket.html#
                                if let Some(packet) = pnet_packet::ethernet::EthernetPacket::owned(packet.to_vec()) {
                                    yield ReceivedPacket {
                                        interface_index: r.interface_index,
                                        ethernet_packet: packet,
                                    }
                                }
                            }
                            Err(e) => {
                                let msg = format!("{}", e);
                                // `Timed out` error should occur in normal cases as the `read_timeout`
                                // configuration param is set to `0`.
                                if &msg != "Timed out" {
                                    panic!("An error occurred while reading: {}", msg);
                                }
                            }
                        }
                    }
                }
            };

            pin_mut!(stream);

            loop {
                while let Some(received_packet) = stream.next().await {
                    let interface = self
                        .interfaces
                        .iter()
                        .find(|&i| i.index == received_packet.interface_index)
                        .expect("should have the network interface");

                    if !Self::should_handle_packet(&received_packet.ethernet_packet, interface) {
                        continue;
                    }

                    match received_packet.ethernet_packet.get_ethertype().0 {
                        ETHERNET_TYPE_IP => {
                            // pnet::packet::ipv4::Ipv4Packet
                            // https://docs.rs/pnet/latest/pnet/packet/ipv4/struct.Ipv4Packet.html
                            if let Some(ipv4) =
                                Ipv4Packet::owned(received_packet.ethernet_packet.packet().to_vec())
                            {
                                println!("ip: {:?}", ipv4);
                                if let Err(e) = self
                                    .sender_ipv4
                                    .send(Ipv4HandlerEvent::ReceivedPacket(ipv4))
                                {
                                    println!("{}", e);
                                }
                            } else {
                                println!("Received a packet whose ETHERNET_TYPE is IP but we couldn't encode it to IPv4 packet.");
                            }
                        }
                        ETHERNET_TYPE_ARP => {
                            // pnet::packet::arp::ArpPacket
                            // https://docs.rs/pnet/latest/pnet/packet/arp/struct.ArpPacket.html
                            if let Some(arp) =
                                ArpPacket::owned(received_packet.ethernet_packet.packet().to_vec())
                            {
                                println!("arp: {:?}", arp);
                                if let Err(e) = self.sender_arp.send(ArpHandlerEvent::ReceivedPacket(arp))
                                {
                                    println!("{}", e);
                                }
                            } else {
                                println!("Received a packet whose ETHERNET_TYPE is ARP but we couldn't encode it to ARP packet.");
                            }
                        }
                        _ => {}
                    }
                }
            }
        };

        tokio::runtime::Handle::current().spawn(fut);
    }

    /// Determine if we handle the packet.
    fn should_handle_packet(
        ethernet_packet: &pnet_packet::ethernet::EthernetPacket,
        interface: &NetworkInterface,
    ) -> bool {
        ethernet_packet.get_destination() == interface.mac.expect("should have mac address")
            || ethernet_packet.get_destination().is_broadcast()
    }
}
