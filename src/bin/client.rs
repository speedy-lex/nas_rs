use std::{io::Write, net::{Ipv4Addr, SocketAddrV4, TcpStream}};

use nas_rs::{ArchivedDirEnum, ArchivedFileRead, DirEnum, FileRead, Request, StructStream, PORT};
use rkyv::rancor::Error;

fn main() {
    // file data
    let file_data = "Hello World!\r\n".as_bytes().to_vec();
    // package data
    let request = Request::EnumDir {
        path: "a".to_string(),
        // len: file_data.len() as u64,
    };

    // connect and send data
    let mut tcp = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't connect");
    let mut stream = StructStream::new(&mut tcp);
    stream.write_struct::<Error>(&request).expect("couldn't send request");
    if let Request::Write { .. } = request {
        stream.write_buffer::<Error>(&file_data).expect("couldn't send file");
    }

    stream.inner.flush().unwrap();
    if let Request::Read { .. } = request {
        let file_info = stream.receive_struct::<FileRead, ArchivedFileRead, Error>().expect("couldn't recieve file");
        let file = stream.receive_buffer::<Error>(file_info.len).expect("couldn't receive file");
        println!("{file:?}");
    } else if let Request::EnumDir { .. } = request {
        let files = stream.receive_struct::<DirEnum, ArchivedDirEnum, Error>().expect("couldn't receive dir enum");
        println!("{files:?}");
    }
}