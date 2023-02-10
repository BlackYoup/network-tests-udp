use std::{
    fs::File,
    io::Write,
    net::{SocketAddr, UdpSocket},
    os::fd::AsRawFd,
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

use bytes::{Buf, BytesMut};
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{debug, error, warn};
use nix::sched::CloneFlags;

use crate::{config::Config, packet::Packet};

pub struct Server {
    config: Config,
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

    fn run_udp_server(&self, log_tx: Sender<Packet>) -> JoinHandle<()> {
        let config = self.config.clone();
        std::thread::spawn(move || {
            let socket = UdpSocket::bind(config.remote).unwrap();
            let mut sequence_recv = 0;
            // Keep the last received remote to detect if the client was restarted
            let mut remote_recv: Option<SocketAddr> = None;

            loop {
                let mut buf: BytesMut = BytesMut::zeroed(config.packet_size);
                let (size, remote) = socket.recv_from(&mut buf).unwrap();
                let received_at = Utc::now();
                debug!(
                    "Received packet: size={}, remote={:?}, content={:?}",
                    size, remote, buf
                );

                if size < config.packet_size {
                    error!("Received packet with length < to what we expect, probably splitted: expected={}, got={}", config.packet_size, size);
                }

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
                    if let Some(old_remote) = remote_recv {
                        if old_remote.port() != remote.port() {
                            // Our client got restarted, reset our sequence
                            sequence_recv = 0;
                            remote_recv = Some(remote);
                        }
                    } else {
                        remote_recv = Some(remote);
                    }
                }

                if sequence != sequence_recv {
                    warn!("Packet with sequence={} is out of order", sequence);
                }

                let packet = Packet {
                    sequence_receiver: sequence_recv,
                    sequence_sender: sequence,
                    received_at,
                    sent_at: date,
                    recv_size: size,
                    remote,
                };

                sequence_recv += 1;
                log_tx.send(packet).unwrap();
            }
        })
    }

    fn run_log_server(&self, rx: Receiver<Packet>) -> JoinHandle<()> {
        let config = self.config.clone();
        debug!("Hello?");
        std::thread::spawn(move || {
            let mut file = File::options()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&config.output_path)
                .expect("Should open output file {}");

            loop {
                match rx.recv() {
                    Ok(packet) => {
                        let msg = format!("{packet}\n");
                        debug!("Writing to file {:?}: {}", config.output_path, msg);
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
