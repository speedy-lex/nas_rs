use std::{io::{Read, Write}, net::{Ipv4Addr, SocketAddrV4, TcpStream}};

use nas_rs::{Request, PORT};
use rkyv::{to_bytes, rancor::Error};

fn main() {
    // file data
    let file_data = "Hello World!\r\n".as_bytes().to_vec();
    // package data
    let file = Request::Read {
        path: "a.txt".to_string(),
        // len: file_data.len() as u64,
    };
    // serialize
    let bytes = to_bytes::<Error>(&file).expect("serialize failed");

    // connect
    let mut stream = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't connect");

    // send length
    stream.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
    // send request
    stream.write_all(&bytes).expect("can't write buf");
    stream.write_all(&file_data).expect("can't write buf");
    stream.flush().unwrap();
    if let Request::Read { .. } = file {
        let mut len = [0; 8];
        stream.read_exact(&mut len).expect("can't read");
        let mut buf = vec![0; u64::from_le_bytes(len) as usize];
        stream.read_exact(&mut buf).expect("can't read");
        println!("{buf:?}");
    }
}