use async_stream::stream;
use futures_util::{pin_mut, StreamExt};
use pnet_datalink::{Config, DataLinkReceiver, NetworkInterface};
use std::time::Duration;

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
                interface_name: i.name.clone(),
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
                                interface_name: r.interface_name.clone(),
                                interface_index: r.interface_index,
                                packet,
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
            println!("{:?}", packet);
        }
    }
}

struct Receiver {
    /// The name of the interface.
    interface_name: String,
    /// The interface index (operating system specific).
    interface_index: u32,
    /// Structure for receiving packets at the data link layer.
    rx: Box<dyn DataLinkReceiver>,
}

#[derive(Debug)]
struct Packet {
    /// The name of the interface.
    interface_name: String,
    /// The interface index (operating system specific).
    interface_index: u32,
    packet: pnet_packet::ethernet::EthernetPacket<'static>,
}

// struct Channel {
//     interface: NetworkInterface,
//     tx: Box<dyn DataLinkSender>,
//     rx: Box<dyn DataLinkReceiver>,
// }
