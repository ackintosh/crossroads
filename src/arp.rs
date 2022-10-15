use pnet_datalink::MacAddr;
use pnet_packet::arp::ArpPacket;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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
}

#[derive(Debug)]
pub(crate) enum ArpEvent {
    ReceivedPacket(ArpPacket<'static>),
    SendArpRequest,
}

struct ArpHandler {
    arp_table: Arc<RwLock<ArpTable>>,
    receiver: UnboundedReceiver<ArpEvent>,
}

impl ArpHandler {
    fn spawn(mut self) {
        let fut = async move {
            loop {
                if let Some(event) = self.receiver.recv().await {
                    match event {
                        ArpEvent::ReceivedPacket(arp_packet) => {
                            println!("TODO: {:?}", arp_packet);
                        }
                        ArpEvent::SendArpRequest => {
                            println!("{:?}", event)
                        }
                    }
                }
            }
        };

        tokio::runtime::Handle::current().spawn(fut);
    }
}

pub(crate) async fn spawn_arp_handler(
    arp_table: Arc<RwLock<ArpTable>>,
) -> UnboundedSender<ArpEvent> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

    ArpHandler {
        arp_table,
        receiver,
    }
    .spawn();

    sender
}
