use std::net::UdpSocket;

use bytes::{BufMut, BytesMut};
use chrono::Utc;
use log::debug;

use crate::config::Config;

pub struct Client {
    config: Config,
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client { config }
    }

    pub fn run(&self) {
        let config = self.config.clone();
        std::thread::spawn(move || {
            let mut sequence: u64 = 0;
            let sleep: u64 = 1_000_000_000 / config.packet_rate as u64;
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Client should bind");

            loop {
                let mut buf = BytesMut::new();
                buf.put_u64(sequence);
                buf.put_u64(Utc::now().timestamp_nanos() as u64);
                buf.resize(config.packet_size, 0);
                let to_send = buf.freeze();

                socket.send_to(&to_send, config.remote).unwrap();
                debug!(
                    "Sent packet size {} to {:?}: {:?}",
                    to_send.len(),
                    config.remote,
                    to_send
                );

                sequence += 1;

                debug!("Sleeping {} before sending next packet", sleep);
                std::thread::sleep(std::time::Duration::from_nanos(sleep));
            }
        })
        .join()
        .unwrap();
    }
}
