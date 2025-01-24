use std::{io::Write, net::{Ipv4Addr, SocketAddrV4, TcpStream}};

use nas_rs::PORT;

fn main() {
    let mut stream = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't connect");
    stream.write_all("Hello World!".as_bytes()).unwrap();
    stream.flush().unwrap();
    println!("done");
}