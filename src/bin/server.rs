use std::{io::{BufRead, BufReader}, net::{Ipv4Addr, SocketAddrV4, TcpListener}};

use nas_rs::PORT;

fn main() {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't bind port");
    for msg in listener.incoming() {
        let msg = msg.unwrap();
        let mut buf = String::new();
        BufReader::new(msg).read_line(&mut buf).unwrap();
        println!("{buf}");
    }
}