use std::{net::{Ipv4Addr, SocketAddrV4, TcpStream}, path::{Path, PathBuf}, str::FromStr};

use iced::{application, widget::{button, column, container, row, text, text_input, vertical_space, Column}, Length, Task};
use iced_aw::number_input;
use openssl::ssl::{SslConnector, SslMethod, SslStream, SslVerifyMode};
use nas_rs::{ArchivedDirEnum, ArchivedFileRead, DirEnum, FileRead, Request, StructStream};
use rancor::{Error, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirEntry {
    name: String,
    is_dir: bool,
}

fn get_stream(address: SocketAddrV4) -> Result<StructStream<SslStream<TcpStream>>, Error> {
    let tcp = TcpStream::connect(address).map_err(Error::new)?;
    let mut ssl = SslConnector::builder(SslMethod::tls_client()).map_err(Error::new)?;
    ssl.set_verify(SslVerifyMode::PEER);
    ssl.set_ca_file("CA.cert").map_err(Error::new)?;
    let ssl = ssl.build();
    let stream = ssl.connect(&address.ip().to_string(), tcp).map_err(Error::new)?;
    Ok(StructStream::new(stream))
}

fn enumerate(address: SocketAddrV4, path: String) -> Result<Vec<DirEntry>, Error> {
    let request = Request::EnumDir { path };
    let mut stream = get_stream(address)?;

    stream.write_struct::<Error>(&request)?;

    let files = stream.receive_struct::<DirEnum, ArchivedDirEnum, Error>().expect("couldn't receive dir enum");
    stream.receive_u64::<Error>()?;
    let files = files.files.into_iter().map(|(path, is_dir)| DirEntry { name: path, is_dir });
    Ok(files.collect())
}
fn download(address: SocketAddrV4, path: String, outpath: &Path) -> Result<(), Error> {
    let request = Request::Read { path };
    let mut stream = get_stream(address)?;

    stream.write_struct::<Error>(&request)?;

    let file = stream.receive_struct::<FileRead, ArchivedFileRead, Error>().expect("couldn't receive dir enum");
    let buf = stream.receive_buffer(file.len)?;
    stream.receive_u64::<Error>()?;
    std::fs::write(outpath, buf).map_err(Error::new)?;
    open::that(outpath.parent().unwrap()).map_err(Error::new)?;
    Ok(())
}
fn upload(address: SocketAddrV4, path: String, inpath: &Path) -> Result<(), Error> {
    let buf = std::fs::read(inpath).map_err(Error::new)?;
    let request = Request::Write { path, len: buf.len() as u64 };
    let mut stream = get_stream(address)?;

    stream.write_struct::<Error>(&request)?;
    stream.write_buffer(&buf)?;

    stream.receive_u64::<Error>()?;
    Ok(())
}
fn delete(address: SocketAddrV4, path: String) -> Result<(), Error> {
    let request = Request::Delete { path };
    let mut stream = get_stream(address)?;

    stream.write_struct::<Error>(&request)?;
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
    Delete(String),
    Download(String),
    Upload,
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
                Message::Upload => {
                    let file_path = if let Some(file_path) = open_file() {
                        file_path
                    } else {
                        return Task::none();
                    };

                    let absolute_path = if path != "." {
                        let mut absolute_path = path.clone();
                        absolute_path.push('/');
                        absolute_path.push_str(file_path.file_name().unwrap().try_into().unwrap());
                        absolute_path
                    } else {
                        TryInto::<&str>::try_into(file_path.file_name().unwrap()).unwrap().to_owned()
                    };
                    upload(*socket, absolute_path, &file_path).unwrap();
                    *needs_update = true;
                }
                Message::Delete(file_name)=>{
                    let absolute_path = if path != "." {
                        let mut absolute_path = path.clone();
                        absolute_path.push('/');
                        absolute_path.push_str(&file_name);
                        absolute_path
                    } else {
                        file_name.clone()
                    };
                    delete(*socket, absolute_path).unwrap();
                    *needs_update = true;
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
                    number_input(port, 0..=u16::MAX, Message::PortInput).ignore_buttons(true).width(Length::Fill),
                    vertical_space().height(Length::Fixed(5.0)),
                    button(text("Connect")).on_press(Message::Connect).width(Length::Fill)
                ).max_width(250).height(Length::Shrink)
            ).center(Length::Fill).into()
        },
        State::Open { path, dir, .. } => {
            let elems = dir.iter().map(|x| {
                let item = if x.is_dir {
                    button(text(x.name.clone())).on_press(Message::Open(x.name.clone()))
                } else {
                    button(text(x.name.clone())).on_press(Message::Download(x.name.clone()))
                };
                row!(
                    item,
                    button(text("delete")).on_press(Message::Delete(x.name.clone()))
                ).into()
            });
            column!(
                text(path),
                button(text("upload")).on_press_with(|| {Message::Upload}),
                button(text("..")).on_press(Message::Open("..".to_string())),
                Column::from_iter(elems)
            ).into()
        },
    }
}

fn open_file() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("Upload a file").pick_file()
}

fn main() {
    let app = application::<State, Message, _, _>("nas_rs client", update, view);
    app.run().unwrap();
}
