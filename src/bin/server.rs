use std::{fs::File, io::{BufReader, BufWriter, Read, Write}, net::{Ipv4Addr, SocketAddrV4, TcpListener}};

use nas_rs::{sanitize_path, ArchivedRequest, Request, PORT};
use rkyv::{access, deserialize, rancor::Error};

fn main() {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).expect("Couldn't bind port");
    for msg in listener.incoming() {
        let mut reader = BufReader::new(msg.as_ref().unwrap());
        let mut writer = BufWriter::new(msg.as_ref().unwrap());

        // read our buffer size
        let mut buf = [0; 4];
        reader.read_exact(&mut buf).unwrap();

        // get serialized struct length
        // unwrap can't fail
        let len = u32::from_le_bytes(buf);

        // read our buffer
        let mut buf = vec![0; len as usize];
        reader.read_exact(&mut buf).unwrap();

        // deserialize
        let file: Request = deserialize::<Request, Error>(access::<ArchivedRequest, Error>(&buf).expect("corrupted file")).expect("corrupted file");
        println!("{file:?}");
        match file {
            Request::Write { path, len } => {
                let mut file_data = vec![0; len as usize];
                reader.read_exact(&mut file_data).expect("buffer not full (buffer sent is not at least len bytes)");
                let path = sanitize_path(&path).expect("not allowed >:(");
                let mut dir = path.clone();
                dir.pop();
                std::fs::create_dir_all(dir).expect("can't create dir");
                File::create(&path).expect("can't create file").write_all(&file_data).expect("can't write");
            },
            Request::Delete { path } => {
                let path = sanitize_path(&path).expect("not allowed >:(");
                if path.is_dir() {
                    std::fs::remove_dir_all(path).expect("can't delete dir");
                } else {
                    std::fs::remove_file(path).expect("can't delete file");
                }
            },
            Request::Read { path } => {
                let path = sanitize_path(&path).expect("not allowed >:(");
                let buf = std::fs::read(path).expect("can't read file");
                writer.write_all(&(buf.len() as u64).to_le_bytes()).expect("can't send");
                writer.write_all(&buf).expect("can't send");
            },
            Request::EnumDir { path } => todo!(),
        }
    }
}