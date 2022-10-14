mod arp;
mod ipv4;

use crate::arp::ArpTable;
use crate::ipv4::{spawn_ipv4_handler, Ipv4HandlerEvent};
use async_stream::stream;
use futures_util::{pin_mut, StreamExt};
use pnet_datalink::{Config, DataLinkReceiver, NetworkInterface};
use pnet_packet::ipv4::Ipv4Packet;
use pnet_packet::Packet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

const ETHERNET_TYPE_IP: u16 = 0x0800;
const ETHERNET_TYPE_ARP: u16 = 0x0806;

#[tokio::main]
async fn main() {
    let interfaces: Vec<NetworkInterface> = pnet_datalink::interfaces()
        .iter()
        .filter(|i| i.mac.is_some() && !i.mac.unwrap().is_zero())
        .cloned()
        .collect();

    println!("*** Detected network interfaces ***");
    for i in &interfaces {
        println!("{:?}", i);
    }

    let config = Config {
        // Specifying read timeout to be `0` in order to let the receiver have non-blocking behavior.
        // https://github.com/libpnet/libpnet/issues/343#issuecomment-406866437
        read_timeout: Some(Duration::from_secs(0)),
        ..Default::default()
    };

    let mut receivers = interfaces
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

    let arp_table = Arc::new(RwLock::new(ArpTable::new()));

    let sender_ipv4 = spawn_ipv4_handler(interfaces.clone(), arp_table.clone()).await;

    loop {
        while let Some(received_packet) = stream.next().await {
            let interface = interfaces
                .iter()
                .find(|&i| i.index == received_packet.interface_index)
                .expect("should have the network interface");

            if !should_handle_packet(&received_packet.ethernet_packet, interface) {
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
                        if let Err(e) = sender_ipv4.send(Ipv4HandlerEvent::ReceivedPacket(ipv4)) {
                            println!("{}", e);
                        }
                    } else {
                        println!("Received a packet whose ETHERNET_TYPE is IP but we couldn't encode it to IPv4 packet.");
                    }
                }
                ETHERNET_TYPE_ARP => {
                    println!("arp");
                }
                _ => {}
            }
        }
    }
}

/// Determine if we handle the packet.
fn should_handle_packet(
    ethernet_packet: &pnet_packet::ethernet::EthernetPacket,
    interface: &NetworkInterface,
) -> bool {
    ethernet_packet.get_destination() == interface.mac.expect("should have mac address")
        || ethernet_packet.get_destination().is_broadcast()
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

// struct Channel {
//     interface: NetworkInterface,
//     tx: Box<dyn DataLinkSender>,
//     rx: Box<dyn DataLinkReceiver>,
// }
