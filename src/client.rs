use std::net::UdpSocket;

use bytes::{BufMut, BytesMut};
use chrono::Utc;
use log::{debug, info};

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
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Client should bind");

            loop {
                if sequence > 0 && sequence % config.packet_rate as u64 == 0 {
                    info!(
                        "Sent {} paquets, waiting 1 second before next batch",
                        sequence
                    );
                    sequence = 0;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
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
            }
        })
        .join()
        .unwrap();
    }
}
