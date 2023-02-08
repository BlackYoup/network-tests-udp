use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    pub packet_rate: usize,
    pub packet_size: usize,
    pub remote: SocketAddr,
    pub output_path: PathBuf,
    pub network_namespace: String,
}

impl Config {
    pub fn new(
        packet_rate: usize,
        packet_size: usize,
        remote: SocketAddr,
        output_path: PathBuf,
        network_namespace: String,
    ) -> Config {
        Config {
            packet_rate,
            packet_size,
            remote,
            output_path,
            network_namespace,
        }
    }
}
