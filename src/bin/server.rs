use std::{env, fs::File, io::Write, net::{Ipv4Addr, SocketAddrV4, TcpListener}, str::FromStr};

use nas_rs::{sanitize_path, ArchivedRequest, DirEnum, FileRead, Request, StructStream, PORT};
use rkyv::rancor::Error;

fn main() {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::from_str(&env::args().nth(1).unwrap_or_else(|| "127.0.0.1".to_string())).unwrap(), PORT)).expect("Couldn't bind port");
    for msg in listener.incoming() {
        let mut tcp = msg.unwrap();
        let mut stream = StructStream::new(&mut tcp);

        // read struct
        let request = stream.receive_struct::<Request, ArchivedRequest, Error>().expect("couldn't receive struct");

        println!("{request:?}");
        match request {
            Request::Write { path, len } => {
                let file_data = stream.receive_buffer::<Error>(len).expect("couldn't receive buffer");
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
                stream.write_struct::<Error>(&FileRead { len: buf.len() as u64 }).expect("couldn't send data");
                stream.write_buffer::<Error>(&buf).expect("couldn't send data");
            },
            Request::EnumDir { path } => {
                let path = sanitize_path(&path).expect("not allowed >:(");
                if !path.is_dir() {
                    panic!("not a dir");
                }
                let contents: Vec<_> = path.read_dir()
                        .expect("couldn't read dir")
                        .map(|x| x.expect("invalid dir entry"))
                        .map(|x| (x.file_name(), x.file_type().expect("invalid file type")))
                        .filter(|(_, t)| !t.is_symlink())
                        .map(|(x, t)| (x.into_string().expect("non utf-8 filename"), t.is_dir()))
                        .collect();
                stream.write_struct::<Error>(&DirEnum {
                    files: contents,
                }).expect("couldn't send dir enum");
            },
        }
    }
}