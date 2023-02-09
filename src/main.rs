mod client;
mod config;
mod packet;
mod server;

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use clap::{value_parser, Arg, ArgAction};
use client::Client;
use config::Config;
use log::{error, info};
use server::Server;

#[derive(Debug, PartialEq)]
enum ProgramRun {
    Client,
    Server,
    Both,
}

fn main() {
    env_logger::init();

    let clap = clap::command!()
        .arg_required_else_help(true)
        .about("A tool to test Out of Order packets")
        .arg(
            Arg::new("rate")
                .long("rate")
                .short('r')
                .help("Define the rate of packets sent per second")
                .default_value("10")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("size")
                .long("size")
                .short('s')
                .help("Packet size in bytes to be sent over the network")
                .default_value("100")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("remote")
                .long("remote")
                .short('e')
                .help("Remote address to send packets to")
                .required(true)
                .value_parser(value_parser!(SocketAddr)),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output file that will print packets received by the receiver")
                .default_value("/root/network-test.log"),
        )
        .arg(
            Arg::new("network-namespace")
                .long("network-namespace")
                .short('n')
                .help("Network namespace in which the server will listen"),
        )
        .arg(
            Arg::new("client")
                .long("client")
                .help("Only run the client")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("server")
                .long("server")
                .help("Only run the server")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let packet_rate = clap.get_one::<usize>("rate").unwrap();
    let packet_size = clap.get_one::<usize>("size").unwrap();
    let remote = clap.get_one::<SocketAddr>("remote").unwrap();
    let output_file = clap.get_one::<String>("output").map(PathBuf::from).unwrap();
    let to_run = match (
        clap.get_one::<bool>("client").unwrap(),
        clap.get_one::<bool>("server").unwrap(),
    ) {
        (true, true) | (false, false) => ProgramRun::Both,
        (true, false) => ProgramRun::Client,
        (false, true) => ProgramRun::Server,
    };

    let network_namespace = clap.get_one::<String>("network-namespace");

    let u64_size = std::mem::size_of::<u64>();
    let min_packet_size = u64_size * 2;
    if *packet_size < min_packet_size {
        error!(
            "Packet should be at least {} bytes in size",
            min_packet_size
        );
        std::process::exit(1);
    }

    if !output_file
        .parent()
        .expect("Output file should be a file into a directory")
        .exists()
    {
        error!("The directory of your output file doesn't exist, please create it");
        std::process::exit(1);
    }

    if to_run == ProgramRun::Both {
        if network_namespace.is_none() {
            error!("--network-namespace is required when running both client and server at the same time");
            std::process::exit(1);
        }
        let network_namespace = network_namespace.as_ref().unwrap();
        if !Path::new(&format!("/run/netns/{network_namespace}")).exists() {
            error!(
                "The network namespace {network_namespace} doesn't exist. Please check the readme"
            );
            std::process::exit(1);
        }
    }

    let config = Config::new(
        *packet_rate,
        *packet_size,
        *remote,
        output_file,
        network_namespace.map(|n| n.into()),
    );

    info!("Running with config: {:#?}", config);
    run(config, to_run);
}

fn run(config: Config, to_run: ProgramRun) {
    let server_thread = if to_run == ProgramRun::Both || to_run == ProgramRun::Server {
        let server = Server::new(config.clone());
        Some(std::thread::spawn(move || {
            server.run();
        }))
    } else {
        None
    };

    let client_thread = if to_run == ProgramRun::Both || to_run == ProgramRun::Client {
        let client = Client::new(config);
        Some(std::thread::spawn(move || {
            client.run();
        }))
    } else {
        None
    };

    if let Some(server_thread) = server_thread {
        server_thread.join().unwrap();
    }

    if let Some(client_thread) = client_thread {
        client_thread.join().unwrap();
    }
}
