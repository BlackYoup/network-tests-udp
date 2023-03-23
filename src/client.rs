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
            let mut start = std::time::Instant::now();
            let mut buf = BytesMut::with_capacity(config.packet_size);

            loop {
                if sequence > 0 && sequence % config.packet_rate as u64 == 0 {
                    info!(
                        "Sent {} paquets in {:?}, waiting 1 second before next batch",
                        sequence, start.elapsed()
                    );
                    sequence = 0;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    start = std::time::Instant::now();
                }
                buf.put_u64(sequence);
                buf.put_u64(Utc::now().timestamp_nanos() as u64);

                socket.send_to(&buf[..], config.remote).unwrap();
                //debug!(
                    //"Sent packet size {} to {:?}: {:?}",
                    //buf.len(),
                    //config.remote,
                    //buf
                //);
                buf.clear();

                sequence += 1;
            }
        })
        .join()
        .unwrap();
    }
}
