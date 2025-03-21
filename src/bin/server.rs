use std::{fs::{read, File}, io::Write, net::{Ipv4Addr, SocketAddrV4, TcpListener}, sync::Arc, thread, time::Duration};

use clap::{arg, value_parser};
use nas_rs::{sanitize_path, sanitize_path_enum, ArchivedRequest, DirEnum, FileRead, Request, StructStream, PORT};
use openssl::{ssl::{Ssl, SslContext, SslContextBuilder, SslFiletype, SslMethod, SslVerifyMode, SslVersion}, x509::X509};
use rkyv::rancor::Error;

fn main() {
    openssl::init();
    let args= clap::Command::new("nas_rs")
        .arg(
            arg!(--ip <ip>)
            .required(false)
            .value_parser(value_parser!(Ipv4Addr))
        ).arg(
            arg!(--port <port>)
            .required(false)
            .value_parser(value_parser!(u16))
        ).get_matches();

    if let Err(err) = std::fs::create_dir("./files/") {
        match err.kind() {
            std::io::ErrorKind::AlreadyExists => {},
            _ => panic!("{err}"),
        }
    }

    let ip = *args.get_one("ip").unwrap_or(&Ipv4Addr::LOCALHOST);
    let server_certificate = X509::from_pem(&read("SERVER.cert").unwrap()).unwrap();
    server_certificate.subject_alt_names().expect("sign the certificate with an ip or domain").iter();
    
    let mut ssl_context = SslContextBuilder::new(SslMethod::tls_server()).unwrap();
    ssl_context.set_min_proto_version(Some(SslVersion::TLS1_3)).unwrap();
    ssl_context.set_ciphersuites("TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256").unwrap();
    ssl_context.set_ca_file("CA.cert").unwrap();
    ssl_context.set_certificate_file("SERVER.cert", SslFiletype::PEM).unwrap();
    ssl_context.set_private_key_file("SERVER.key", SslFiletype::PEM).unwrap();
    ssl_context.set_verify(SslVerifyMode::PEER);
    let ssl_context = Arc::new(ssl_context.build());
    
    let mut threads = vec![];
    let listener = TcpListener::bind(SocketAddrV4::new(ip, *args.get_one("port").unwrap_or(&PORT))).expect("Couldn't bind port");
    for msg in listener.incoming() {
        let ssl_ref = ssl_context.clone();
        let thread = thread::spawn(move || handle_connection(msg, ssl_ref));
        threads.push(thread);
        threads = threads.into_iter().filter_map(|x| {
            if !x.is_finished() {
                return Some(x);
            }
            if let Some(err) = x.join().err() {
                eprintln!("{err:?}");
            }
            None
        }).collect();
    }
}

fn handle_connection(msg: Result<std::net::TcpStream, std::io::Error>, ctx: Arc<SslContext>) {
    let tcp = msg.unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(500))).unwrap();
    let mut ssl = Ssl::new(&ctx).unwrap().accept(tcp).unwrap();
    let mut stream = StructStream::new(&mut ssl);

    // read struct
    let request = stream.receive_struct::<Request, ArchivedRequest, Error>().expect("couldn't receive struct");

    println!("{request:?}");
    match request {
        Request::Write { path, len } => {
            let file_data = stream.receive_buffer::<Error>(len).expect("couldn't receive buffer");
            let path = sanitize_path(&path).expect("not allowed >:(");
            File::create(&path).expect("can't create file").write_all(&file_data).expect("can't write");
        },
        Request::MkDir { path } => {
            let path = sanitize_path(&path).expect("not allowed >:(");
            std::fs::create_dir(path).unwrap();
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
            let path = sanitize_path_enum(&path).expect("not allowed >:(");
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
    stream.write_u64::<Error>(0).unwrap();
}