use std::{io::{Read, Write}, net::{Ipv4Addr, SocketAddrV4, TcpStream}};

use clap::{arg, value_parser};
use nas_rs::{ArchivedDirEnum, ArchivedFileRead, DirEnum, FileRead, Request, StructStream, PORT};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use rkyv::rancor::Error;

fn main() {
    let args= clap::Command::new("nas_rs")
        .arg(
            arg!([file_path])
            .required(true)
            .value_parser(value_parser!(String))
        ).arg(
            arg!(--write)
            .required(false)
        ).arg(
            arg!(--mkdir)
            .required(false)
        ).arg(
            arg!(--delete)
            .required(false)
        ).arg(
            arg!(--enumerate)
            .required(false)
        ).arg(
            arg!(--ip <ip>)
            .required(false)
            .value_parser(value_parser!(Ipv4Addr))
        ).arg(
            arg!(--port <port>)
            .required(false)
            .value_parser(value_parser!(u16))
        ).get_matches();
    
    let path = args.get_one::<String>("file_path").unwrap().clone();

    // file data
    let file_data = args.get_one("write")
        .filter(|x| **x)
        .map(|_: &bool| {
            let mut vec = vec![];
            std::io::stdin().read_to_end(&mut vec).expect("can't read");
            vec
        }).unwrap_or_default();
    // package data
    let request = args.get_one("delete")
        .filter(|x| **x)
        .map(|_: &bool| {
            Request::Delete { path: path.clone() }
        })
        .unwrap_or_else(|| {
            args.get_one("write")
            .filter(|x| **x)
            .map(|_: &bool| {
                Request::Write { path: path.clone(), len: file_data.len() as u64 }
            }).unwrap_or_else(|| {
                args.get_one("mkdir")
                .filter(|x| **x)
                .map(|_: &bool| {
                    Request::MkDir { path: path.clone() }
                }).unwrap_or_else(|| {
                    args.get_one("enumerate").filter(|x| **x)
                    .map(|_: &bool| {
                        Request::EnumDir { path: path.clone() }
                    })
                    .unwrap_or(Request::Read { path })
                })
            })
        });

    // connect and send data
    let ip = *args.get_one("ip").unwrap_or(&Ipv4Addr::LOCALHOST);
    let tcp = TcpStream::connect(SocketAddrV4::new(ip, *args.get_one("port").unwrap_or(&PORT))).expect("Couldn't connect");
    let mut ssl = SslConnector::builder(SslMethod::tls_client()).unwrap();
    ssl.set_verify(SslVerifyMode::PEER);
    ssl.set_ca_file("CA.cert").unwrap();
    let ssl = ssl.build();
    let mut stream = ssl.connect(&ip.to_string(), tcp).unwrap();
    let mut stream = StructStream::new(&mut stream);
    stream.write_struct::<Error>(&request).expect("couldn't send request");
    if let Request::Write { .. } = request {
        stream.write_buffer::<Error>(&file_data).expect("couldn't send file");
    }

    stream.inner.flush().unwrap();
    if let Request::Read { .. } = request {
        let file_info = stream.receive_struct::<FileRead, ArchivedFileRead, Error>().expect("couldn't recieve file");
        let file = stream.receive_buffer::<Error>(file_info.len).expect("couldn't receive file");
        std::io::stdout().write_all(&file).unwrap();
    } else if let Request::EnumDir { .. } = request {
        let files = stream.receive_struct::<DirEnum, ArchivedDirEnum, Error>().expect("couldn't receive dir enum");
        println!("{files:?}");
    }
}