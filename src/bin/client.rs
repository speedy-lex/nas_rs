use std::{fs::File, io::{Read, Write}, net::{Ipv4Addr, SocketAddrV4, TcpStream}};

use clap::{arg, value_parser};
use nas_rs::{ArchivedDirEnum, ArchivedFileRead, DirEnum, FileRead, Request, StructStream, PORT};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use rkyv::rancor::Error;

fn main() {
    openssl::init();
    let args= clap::Command::new("nas_rs")
        .arg(
            arg!([file_path])
            .required(true)
            .value_parser(value_parser!(String))
        ).arg(
            arg!(--write)
            .required(false)
            .requires("in")
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
        ).arg(
            arg!(--in <in_file>)
        ).arg(
            arg!(--out <out_file>)
            .required_unless_present_any(["write", "mkdir", "delete", "enumerate"])
        ).get_matches();
    
    let path = args.get_one::<String>("file_path").unwrap().clone();

    // file data
    let file_data = args.get_one("write")
        .filter(|x| **x)
        .map(|_: &bool| {
            let mut vec = vec![];
            let mut file = args.get_one("in").map(|x: &String| Box::new(File::open(x).unwrap()) as Box<dyn Read>).unwrap_or_else(|| Box::new(std::io::stdin()));
            file.read_to_end(&mut vec).expect("can't read");
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
        println!("done writing");
    }

    stream.inner.flush().unwrap();
    if let Request::Read { .. } = request {
        let file_info = stream.receive_struct::<FileRead, ArchivedFileRead, Error>().expect("couldn't recieve file");
        let file = stream.receive_buffer::<Error>(file_info.len).expect("couldn't receive file");
        let mut out_file = args.get_one("out").map(|x: &String| Box::new(File::create(x).unwrap()) as Box<dyn Write>).unwrap_or_else(|| Box::new(std::io::stdout()));
        out_file.write_all(&file).unwrap();
        out_file.flush().unwrap();
    } else if let Request::EnumDir { .. } = request {
        let files = stream.receive_struct::<DirEnum, ArchivedDirEnum, Error>().expect("couldn't receive dir enum");
        println!("{files:?}");
    }

    stream.receive_u64::<Error>().unwrap();
}