mod arp;
mod ethernet;
mod ipv4;

use crate::arp::{spawn_arp_handler, ArpHandlerEvent, ArpTable};
use crate::ethernet::{spawn_ethernet_handler, EthernetHandlerEvent};
use crate::ipv4::{spawn_ipv4_handler, Ipv4HandlerEvent};
use pnet_datalink::NetworkInterface;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use tokio::signal::unix::{signal, Signal, SignalKind};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    info!("                                    /////////                                         ");
    info!("                                    /////////                                         ");
    info!("                                    /////////                                         ");
    info!("//////////////////////////////////////////////////////////////////////////////////////");
    info!(
        "////////////////////////////// Crossroads v{} /////////////////////////////////////",
        env!("CARGO_PKG_VERSION")
    );
    info!("//////////////////////////////////////////////////////////////////////////////////////");
    info!("                                    /////////                                         ");
    info!("                                    /////////                                         ");
    info!("                                    /////////                                         ");

    let interfaces: Vec<NetworkInterface> = pnet_datalink::interfaces()
        .iter()
        .filter(|i| i.mac.is_some() && !i.mac.unwrap().is_zero())
        .cloned()
        .collect();

    info!("***********************************");
    info!("*** Detected network interfaces ***");
    info!("***********************************");
    for i in &interfaces {
        info!("{:?}", i);
    }

    let arp_table = Arc::new(RwLock::new(ArpTable::new()));
    let (sender_ethernet, receiver_ethernet) = tokio::sync::mpsc::unbounded_channel();
    let (sender_arp, receiver_arp) = tokio::sync::mpsc::unbounded_channel();
    let (sender_ipv4, receiver_ipv4) = tokio::sync::mpsc::unbounded_channel();

    // Spawn packet handlers.
    let jh_ethernet = spawn_ethernet_handler(
        &interfaces,
        receiver_ethernet,
        sender_arp.clone(),
        sender_ipv4.clone(),
    )
    .await;
    let jh_arp = spawn_arp_handler(&interfaces, arp_table.clone(), receiver_arp).await;
    let jh_ipv4 = spawn_ipv4_handler(
        interfaces.clone(),
        arp_table.clone(),
        receiver_ipv4,
        sender_arp.clone(),
    )
    .await;

    // Block the current thread until a shutdown signal is received.
    let message = tokio::runtime::Handle::current()
        .spawn(async {
            let mut handles = vec![];

            match signal(SignalKind::terminate()) {
                Ok(terminate_stream) => {
                    let terminate = SignalFuture::new(terminate_stream, "Received SIGTERM");
                    handles.push(terminate);
                }
                Err(e) => error!("Could not register SIGTERM handler: {}", e),
            }

            match signal(SignalKind::interrupt()) {
                Ok(interrupt_stream) => {
                    let interrupt = SignalFuture::new(interrupt_stream, "Received SIGINT");
                    handles.push(interrupt);
                }
                Err(e) => error!("Could not register SIGINT handler: {}", e),
            }

            futures_util::future::select_all(handles.into_iter()).await
        })
        .await
        .unwrap();

    // TODO: Graceful shutdown on each handlers.
    info!("{:?}. Starting shutdown process...", message.0);
    sender_ethernet
        .send(EthernetHandlerEvent::Shutdown)
        .unwrap();
    sender_arp.send(ArpHandlerEvent::Shutdown).unwrap();
    sender_ipv4.send(Ipv4HandlerEvent::Shutdown).unwrap();

    macro_rules! log_if_error {
        ($result:expr) => {
            if let Err(e) = $result {
                error!("{:?}", e);
            }
        };
    }
    let (eth, arp, ipv4) = futures_util::join!(jh_ethernet, jh_arp, jh_ipv4);
    log_if_error!(eth);
    log_if_error!(arp);
    log_if_error!(ipv4);

    info!("Done.");
}

// struct Channel {
//     interface: NetworkInterface,
//     tx: Box<dyn DataLinkSender>,
//     rx: Box<dyn DataLinkReceiver>,
// }

// cf. https://github.com/sigp/lighthouse/blob/d9910f96c5f71881b88eec15253b31890bcd28d2/lighthouse/environment/src/lib.rs#L492
#[cfg(target_family = "unix")]
pub(crate) struct SignalFuture {
    signal: Signal,
    message: &'static str,
}

#[cfg(target_family = "unix")]
impl SignalFuture {
    pub fn new(signal: Signal, message: &'static str) -> SignalFuture {
        SignalFuture { signal, message }
    }
}

#[cfg(target_family = "unix")]
impl Future for SignalFuture {
    type Output = Option<&'static str>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.signal.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(_)) => Poll::Ready(Some(self.message)),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}
