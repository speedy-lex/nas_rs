use std::{io::Write, net::{Ipv4Addr, SocketAddrV4, TcpStream}};

use nas_rs::{Request, PORT};
use rkyv::{to_bytes, rancor::Error};

fn main() {
    // package data
    let file = Request::Write {
        path: "Hello World!".to_string(),
        len: 16,
    };
    // serialize
    let bytes = to_bytes::<Error>(&file).expect("serialize failed");
    // file data
    let file_data: Vec<_> = (0..16).collect();

    // connect
    let mut stream = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't connect");

    // send length
    stream.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
    // send request
    stream.write_all(&bytes).expect("can't write buf");
    stream.write_all(&file_data).expect("can't write buf");
    stream.flush().unwrap();
}