use std::{fs::File, io::{BufReader, Read, Write}, net::{Ipv4Addr, SocketAddrV4, TcpListener}, path::PathBuf};

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
        println!("{file:?}");
        match file {
            Request::Write { path, len } => {
                let mut file_data = vec![0; len as usize];
                msg.read_exact(&mut file_data).expect("buffer not full (buffer sent is not at least len bytes)");
                println!("{file_data:?}");
                let path = PathBuf::from(path);
                if path.iter().any(|x| x == "..") || path.is_absolute() {
                    panic!("not allowed >:(");
                }
                let mut dir = path.clone();
                dir.pop();
                std::fs::create_dir_all(dir).expect("can't create dir");
                File::create(&path).expect("can't create file").write_all(&file_data).expect("can't write");
            },
            Request::Delete { path } => {
                
            },
        }
    }
}