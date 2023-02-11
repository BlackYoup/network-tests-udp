use std::{fmt::Display, net::SocketAddr};

use chrono::{DateTime, Utc};

pub struct Packet {
    pub sequence_sender: u64,
    pub sequence_receiver: u64,
    pub packet_number: u64,
    pub sent_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub remote: SocketAddr,
    pub recv_size: usize,
}

impl Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let diff = self.received_at - self.sent_at;
        let seq_diff = if self.sequence_sender != self.sequence_receiver {
            "y"
        } else {
            "n"
        };
        write!(
            f,
            "ooo={}, pck_nbr={}, seq_sent={}, seq_recv={}, sent_at={}, recv_at={}, size={}, time_ms={}ms, time_ns={}ns, remote={}:{}",
            seq_diff,
            self.packet_number,
            self.sequence_sender,
            self.sequence_receiver,
            self.sent_at.format("%H:%M:%S%.f"),
            self.received_at.format("%H:%M:%S%.f"),
            self.recv_size,
            diff.num_nanoseconds().unwrap() as f64 / 1_000_000_f64,
            diff.num_nanoseconds().unwrap(),
            self.remote.ip(),
            self.remote.port()
        )
    }
}
