use std::{io::{BufReader, Read}, net::{Ipv4Addr, SocketAddrV4, TcpListener}};

use nas_rs::{ArchivedRequest, Request, PORT};
use rkyv::{access, deserialize, rancor::Error};

fn main() {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't bind port");
    for msg in listener.incoming() {
        let mut msg = BufReader::new(msg.unwrap());

        // read our buffer size
        let mut buf = [0; 4];
        msg.read_exact(&mut buf).unwrap();

        // get serialized struct length
        // unwrap can't fail
        let len = u32::from_le_bytes(buf);

        // read our buffer
        let mut buf = vec![0; len as usize];
        msg.read_exact(&mut buf).unwrap();

        // deserialize
        let file: Request = deserialize::<Request, Error>(access::<ArchivedRequest, Error>(&buf).expect("corrupted file")).expect("corrupted file");
        println!("{file:?}")
    }
}