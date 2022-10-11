use async_stream::stream;
use futures_util::{pin_mut, StreamExt};
use pnet_datalink::{Config, DataLinkReceiver, NetworkInterface};
use std::time::Duration;

const ETHERNET_TYPE_IP: u16 = 0x0800;
const ETHERNET_TYPE_ARP: u16 = 0x0806;
const ETHERNET_TYPE_IPV6: u16 = 0x86dd;

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
                        if let Some(packet) = pnet_packet::ethernet::EthernetPacket::owned(packet.to_vec()) {
                            yield Packet {
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
        while let Some(packet) = stream.next().await {
            let interface = interfaces
                .iter()
                .find(|&i| i.index == packet.interface_index)
                .expect("should have the network interface");

            if !should_handle_packet(&packet.ethernet_packet, interface) {
                continue;
            }

            match packet.ethernet_packet.get_ethertype().0 {
                ETHERNET_TYPE_IP => {
                    println!("ip");
                }
                ETHERNET_TYPE_ARP => {
                    println!("arp");
                }
                ETHERNET_TYPE_IPV6 => {
                    println!("ipv6");
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
struct Packet {
    /// The interface index (operating system specific).
    interface_index: u32,
    ethernet_packet: pnet_packet::ethernet::EthernetPacket<'static>,
}

// struct Channel {
//     interface: NetworkInterface,
//     tx: Box<dyn DataLinkSender>,
//     rx: Box<dyn DataLinkReceiver>,
// }
