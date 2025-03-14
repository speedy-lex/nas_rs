use std::{net::{Ipv4Addr, SocketAddrV4, TcpStream}, path::Path, str::FromStr};

use iced::{application, widget::{button, column, container, text, text_input, vertical_space, Column}, Length, Task};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use nas_rs::{ArchivedDirEnum, ArchivedFileRead, DirEnum, FileRead, Request, StructStream};
use rancor::{Error, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirEntry {
    name: String,
    is_dir: bool,
}

fn enumerate(address: SocketAddrV4, path: String) -> Result<Vec<DirEntry>, Error> {
    let is_root = path == ".";
    let request = Request::EnumDir { path };

    let tcp = TcpStream::connect(address).expect("Couldn't connect");
    let mut ssl = SslConnector::builder(SslMethod::tls_client()).map_err(Error::new)?;
    ssl.set_verify(SslVerifyMode::PEER);
    ssl.set_ca_file("CA.cert").map_err(Error::new)?;
    let ssl = ssl.build();
    let mut stream = ssl.connect(&address.ip().to_string(), tcp).map_err(Error::new)?;
    let mut stream = StructStream::new(&mut stream);

    stream.write_struct::<Error>(&request)?;

    let files = stream.receive_struct::<DirEnum, ArchivedDirEnum, Error>().expect("couldn't receive dir enum");
    stream.receive_u64::<Error>()?;
    let files = files.files.into_iter().map(|(path, is_dir)| DirEntry { name: path, is_dir });
    if is_root {
        Ok(files.collect())
    } else {
        Ok([DirEntry { is_dir: true, name: "..".to_string() }].into_iter().chain(files).collect())
    }
}
fn download(address: SocketAddrV4, path: String, outpath: &Path) -> Result<(), Error> {
    let request = Request::Read { path };

    let tcp = TcpStream::connect(address).expect("Couldn't connect");
    let mut ssl = SslConnector::builder(SslMethod::tls_client()).map_err(Error::new)?;
    ssl.set_verify(SslVerifyMode::PEER);
    ssl.set_ca_file("CA.cert").map_err(Error::new)?;
    let ssl = ssl.build();
    let mut stream = ssl.connect(&address.ip().to_string(), tcp).map_err(Error::new)?;
    let mut stream = StructStream::new(&mut stream);

    stream.write_struct::<Error>(&request)?;

    let file = stream.receive_struct::<FileRead, ArchivedFileRead, Error>().expect("couldn't receive dir enum");
    let buf = stream.receive_buffer(file.len)?;
    stream.receive_u64::<Error>()?;
    std::fs::write(outpath, buf).map_err(Error::new)?;
    open::that(outpath.parent().unwrap()).map_err(Error::new)?;
    Ok(())
}


#[derive(Debug)]
enum State {
    Login {
        ip: String,
        bad_ip: bool,
        port: u16,
    },
    Open {
        socket: SocketAddrV4,
        path: String,
        needs_update: bool,
        dir: Vec<DirEntry>,
    },
}
impl Default for State {
    fn default() -> Self {
        Self::Login { ip: String::new(), bad_ip: false, port: nas_rs::PORT }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    IpInput(String),
    PortInput(u16),
    Connect,
    Open(String),
    Download(String),
}

fn update(state: &mut State, msg: Message) -> iced::Task<Message> {
    match state {
        State::Login { ip: str_ip, bad_ip, port } => {
            match msg {
                Message::IpInput(new_ip) => {
                    *str_ip = new_ip;
                    *bad_ip = false;
                },
                Message::PortInput(new_port) => *port = new_port,
                Message::Connect => {
                    let ip = Ipv4Addr::from_str(str_ip);
                    if ip.is_err() {
                        *bad_ip = true;
                        str_ip.clear();
                        return Task::none();
                    }
                    let ip = ip.unwrap();
                    *state = State::Open { socket: SocketAddrV4::new(ip, *port), path: '.'.to_string(), needs_update: true, dir: vec![] };
                    return update(state, msg);
                },
                _ => panic!("invalid message")
            }
        },
        State::Open { path, socket, needs_update, dir } => {
            match msg {
                Message::Open(open) => {
                    if open == ".." {
                        if path.contains('/') {
                            *path = path.rsplit_once('/').unwrap().0.to_string()
                        } else {
                            *path = '.'.to_string()
                        }
                    } else if path == "." {
                        *path = open;
                    } else {
                        path.push('/');
                        path.push_str(&open);
                    }
                    *needs_update = true;
                },
                Message::Download(file_name) => {
                    let absolute_path = if path != "." {
                        let mut absolute_path = path.clone();
                        absolute_path.push('/');
                        absolute_path.push_str(&file_name);
                        absolute_path
                    } else {
                        file_name.clone()
                    };
                    let mut outpath = std::env::current_dir().unwrap();
                    outpath.push("downloads");
                    outpath.push(file_name);
                    download(*socket, absolute_path, &outpath).unwrap();
                },
                Message::Connect => {},
                _ => {
                    panic!("invalid message");
                }
            }
            if *needs_update {
                *dir = enumerate(*socket, path.clone()).unwrap();
                *needs_update = false;
            }
        },
    }
    Task::none()
}
fn view(state: &State) -> iced::Element<Message> {
    match state {
        State::Login { ip, port, bad_ip } => {
            container(
                column!(
                    text_input(if *bad_ip {"Invalid Ip"} else {"Ipv4 address"}, ip).on_input(Message::IpInput),
                    vertical_space().height(Length::Fixed(5.0)),
                    text_input("Port", &port.to_string()).on_input(|x| {Message::PortInput(if x.is_empty() {0} else {x.parse().unwrap_or(*port)})}),
                    vertical_space().height(Length::Fixed(5.0)),
                    button(text("Connect")).on_press(Message::Connect).width(Length::Fill)
                ).max_width(250).height(Length::Shrink)
            ).center(Length::Fill).into()
        },
        State::Open { path, dir, .. } => {
            let elems = dir.iter().map(|x| {
                if x.is_dir {
                    button(text(x.name.clone())).on_press(Message::Open(x.name.clone())).into()
                } else {
                    button(text(x.name.clone())).on_press(Message::Download(x.name.clone())).into()
                }
            });
            column!(
                text(path),
                Column::from_iter(elems)
            ).into()
        },
    }
}

fn main() {
    let app = application::<State, Message, _, _>("hello", update, view);
    app.run().unwrap();
}
