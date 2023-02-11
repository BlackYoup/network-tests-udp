use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    net::{SocketAddr, UdpSocket},
    os::fd::AsRawFd,
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
    time::Instant,
};

use bytes::{Buf, BytesMut};
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{debug, error, trace, warn};
use nix::sched::CloneFlags;

use crate::{config::Config, packet::Packet};

pub struct Server {
    config: Config,
}

pub enum Message {
    Packet(Packet),
    Lost(u64),
    Reset,
}

impl Server {
    pub fn new(config: Config) -> Server {
        Server { config }
    }

    pub fn run(&self) {
        self.setup_ns();
        let (tx, rx) = channel();
        let handle_udp = self.run_udp_server(tx);
        let handle_log_server = self.run_log_server(rx);

        handle_log_server.join().unwrap();
        handle_udp.join().unwrap();
    }

    fn setup_ns(&self) {
        if let Some(ns) = &self.config.network_namespace {
            let file = File::open(format!("/run/netns/{}", ns))
                .unwrap_or_else(|_| panic!("{} ns should exist", ns));

            let fd = file.as_raw_fd();
            nix::sched::setns(fd, CloneFlags::CLONE_NEWNET)
                .expect("Should reschedule thread into namespace");
        }
    }

    fn run_udp_server(&self, log_tx: Sender<Message>) -> JoinHandle<()> {
        let config = self.config.clone();
        std::thread::spawn(move || {
            let socket = UdpSocket::bind(config.remote).unwrap();
            // Expected next sequence
            let mut sequence_recv = 0;
            // Keep the last received remote to detect if the client was restarted
            let mut remote_recv: Option<SocketAddr> = None;
            let mut loss: HashMap<u64, Instant> = HashMap::new();

            loop {
                let mut buf: BytesMut = BytesMut::zeroed(65536);
                let (size, remote) = socket.recv_from(&mut buf).unwrap();
                let received_at = Utc::now();
                debug!("Received packet: size={}, remote={:?}", size, remote);
                trace!("content={:?}", buf);

                // Print those lost packets once we received a new one but
                // before we start printing the new ones
                // Consider we lost the packets after 500ms
                loss.retain(|&lost_seq, lost_at| {
                    if lost_at.elapsed() > std::time::Duration::from_millis(500) {
                        warn!("Packet with sequence {lost_seq} has been lost");
                        log_tx.send(Message::Lost(lost_seq)).unwrap();
                        false
                    } else {
                        true
                    }
                });

                let mut buffer = buf.freeze();
                let sequence = buffer.get_u64();
                let date = buffer.get_u64();
                let secs = date / 1_000_000_000;
                let nanos = date % 1_000_000_000;
                let date = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp_opt(secs as i64, nanos as u32).unwrap(),
                    Utc,
                );

                if sequence == 0 {
                    // Our client ended its previous chunk, reset our sequence
                    sequence_recv = 0;
                    if let Some(old_remote) = remote_recv {
                        if old_remote.port() != remote.port() {
                            remote_recv = Some(remote);
                            log_tx.send(Message::Reset).unwrap();
                        }
                    } else {
                        remote_recv = Some(remote);
                    }
                }

                debug!("Packet: sequence={sequence}, sequence_recv={sequence_recv}");

                let packet = Packet {
                    sequence_receiver: sequence_recv,
                    sequence_sender: sequence,
                    received_at,
                    sent_at: date,
                    recv_size: size,
                    remote,
                };

                if sequence != sequence_recv {
                    // We just received a packet that was missing, do not treat as lost
                    // We don't increase our sequence since we already increased it in the else{} block below
                    if loss.contains_key(&sequence) {
                        warn!("Packet with sequence={} is out of order", sequence);
                        loss.remove(&sequence).unwrap();
                    } else {
                        // Compute the number of losses
                        // If we are here, it means that we received a higher sequence than our current sequence
                        // Otherwise we would have ended up into the previous if{} block
                        let total_losses = sequence - sequence_recv;
                        debug!("Total losses={total_losses}, sequence={sequence}, sequence_recv={sequence_recv}");
                        if total_losses > 1_000_000 {
                            panic!("Underflow occuring");
                        }

                        for i in 0..total_losses {
                            loss.insert(sequence_recv + i, Instant::now());
                        }

                        // Let's increase or sequence so that following packet do not appear as Out of Order
                        sequence_recv += total_losses;
                        sequence_recv += 1;
                    }
                } else {
                    sequence_recv += 1;
                };

                log_tx.send(Message::Packet(packet)).unwrap();
            }
        })
    }

    fn run_log_server(&self, rx: Receiver<Message>) -> JoinHandle<()> {
        let config = self.config.clone();
        std::thread::spawn(move || {
            let mut file = File::options()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&config.output_path)
                .expect("Should open output file {}");

            loop {
                match rx.recv() {
                    Ok(Message::Packet(packet)) => {
                        let msg = format!("{packet}\n");
                        debug!("Writing to file {:?}: {}", config.output_path, msg);
                        file.write_all(msg.as_bytes()).unwrap();
                        file.flush().unwrap();
                    }
                    Ok(Message::Reset) => {
                        file.set_len(0).unwrap();
                    }
                    Ok(Message::Lost(seq)) => {
                        let msg = format!("lost=yes, seq_recv={seq}\n");
                        file.write_all(msg.as_bytes()).unwrap();
                        file.flush().unwrap();
                    }
                    Err(err) => {
                        error!("Got error while recv() from Channel: {:?}", err);
                        break;
                    }
                }
            }
        })
    }
}
