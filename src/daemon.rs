use clap::Parser;
use std::error::Error;

pub mod schema;
pub mod arguments;
pub mod interface;
pub mod configfile;
pub mod database_utils;

use interface::init;
use arguments::Args;
use configfile::Config;

use nng::{options::{protocol::pubsub::Subscribe, Options}, Protocol, Socket};

const SOCKET_HOST: &str = "127.0.0.1";
const SOCKET_SUB_PORT: i16 = 5556;
const SOCKET_PUB_PORT: i16 = 5555;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let config: Config = Config::from_file(args.config);

    init(config.database_path)?;

    let sub = Socket::new(Protocol::Sub0).unwrap();
    let pub_sock = Socket::new(Protocol::Pub0).unwrap();

    sub.listen(&format!("tcp://{}:{}", SOCKET_HOST, SOCKET_SUB_PORT)).unwrap();
    pub_sock.listen(&format!("tcp://{}:{}", SOCKET_HOST, SOCKET_PUB_PORT)).unwrap();

    sub.set_opt::<Subscribe>(b"".to_vec()).unwrap();

    loop {
        let msg = sub.recv().unwrap();
        pub_sock.send(msg).unwrap();
    }
}
