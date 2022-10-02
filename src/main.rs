fn main() {
    let interfaces = pnet_datalink::interfaces();
    for i in interfaces {
        println!("{:?}", i);
    }

    let interface = interfaces
        .iter()
        .filter(|i| i.name == "en0")
        .next()
        .expect("should have an interface");

    let (mut _tx, mut rx) = match pnet_datalink::channel(interface, Default::default()) {
        Ok(pnet_datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!(
            "An error occurred when creating the datalink channel: {}",
            e
        ),
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                let packet = pnet_packet::ethernet::EthernetPacket::new(packet);
                println!("{:?}", packet);
            }
            Err(e) => {
                panic!("An error occurred while reading: {}", e);
            }
        }
    }
}
