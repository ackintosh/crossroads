mod arp;
mod ethernet;
mod ipv4;

use crate::arp::{spawn_arp_handler, ArpTable};
use crate::ethernet::spawn_ethernet_handler;
use crate::ipv4::spawn_ipv4_handler;
use pnet_datalink::NetworkInterface;
use std::sync::{Arc, RwLock};

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

    let arp_table = Arc::new(RwLock::new(ArpTable::new()));
    let (sender_ethernet, receiver_ethernet) = tokio::sync::mpsc::unbounded_channel();
    let (sender_arp, receiver_arp) = tokio::sync::mpsc::unbounded_channel();
    let (sender_ipv4, receiver_ipv4) = tokio::sync::mpsc::unbounded_channel();

    spawn_ethernet_handler(
        &interfaces,
        receiver_ethernet,
        sender_arp.clone(),
        sender_ipv4.clone(),
    )
    .await;
    spawn_arp_handler(&interfaces, arp_table.clone(), receiver_arp).await;
    spawn_ipv4_handler(
        interfaces.clone(),
        arp_table.clone(),
        receiver_ipv4,
        sender_arp.clone(),
    )
    .await;

    loop {
    }
}

// struct Channel {
//     interface: NetworkInterface,
//     tx: Box<dyn DataLinkSender>,
//     rx: Box<dyn DataLinkReceiver>,
// }
