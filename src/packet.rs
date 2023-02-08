use std::{fmt::Display, net::SocketAddr};

use chrono::{DateTime, Utc};

pub struct Packet {
    pub sequence_sender: u64,
    pub sequence_receiver: u64,
    pub sent_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub remote: SocketAddr,
    pub recv_size: usize,
}

impl Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let diff = self.received_at - self.sent_at;
        write!(
            f,
            "recv_at={}, seq_recv={}, sent_at={}, seq_sent={}, size={}, time_ms={}ms, time_ns={}ns, remote={}:{}",
            self.received_at.format("%H:%M:%S%.f"),
            self.sequence_receiver,
            self.sent_at.format("%H:%M:%S%.f"),
            self.sequence_sender,
            self.recv_size,
            diff.num_nanoseconds().unwrap() as f64 / 1_000_000_f64,
            diff.num_nanoseconds().unwrap(),
            self.remote.ip(),
            self.remote.port()
        )
    }
}
