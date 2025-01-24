use std::{io::Write, net::{Ipv4Addr, SocketAddrV4, TcpStream}};

use nas_rs::{Request, PORT};
use rkyv::{to_bytes, rancor::Error};

fn main() {
    // package data
    let file = Request {
        str: "Hello World!".to_string(),
    };
    // serialize
    let bytes = to_bytes::<Error>(&file).expect("serialize failed");
    let mut stream = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't connect"); // connect to server
    stream.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
    stream.write_all(&bytes).expect("can't write buf");
    stream.flush().unwrap();
    println!("done");
}