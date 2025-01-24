use std::{fs::{self, create_dir_all, File}, io::{BufReader, Read, Write}, net::{Ipv4Addr, SocketAddrV4, TcpListener}, path::{Path, PathBuf}, str::FromStr};

use nas_rs::{ArchivedRequest, Request, PORT};
use rkyv::{access, deserialize, rancor::Error};

#[derive(Debug)]
enum Commands {
    Add,
    Remove,
    NA
}
impl Commands {
    fn from_str(command: &str) -> Option<Commands> {
        match command.split_ascii_whitespace().next() {
            Some("add") => Some(Commands::Add),
            Some("remove") => Some(Commands::Remove),
            _ => Some(Commands::NA),
        }
    }
    fn execute(&self, data: &Request, info: &Request) {
        println!("{:?}",self);
        match self {
            Commands::Add => {
                let filen: &std::ffi::OsStr = Path::new(&info.str).file_name().unwrap();
                let path = Path::new(".").join("files").join(filen);

                let mut file = File::create(path).unwrap();

                file.write_all(data.str.as_bytes()).unwrap();
                file.flush().unwrap();
            },
            Commands::Remove => {
                for entry in std::fs::read_dir("./files").unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    
                    let file_name: Vec<&str> = info.str.split_ascii_whitespace().collect();
                    let binding_shit = PathBuf::from_str(file_name[1]).expect("failed");
                    let second_side_of_path = binding_shit.file_name().unwrap();

                    if path.file_name().unwrap() == second_side_of_path {
                        fs::remove_file(path).unwrap();
                        break;
                    }
                }
            }
            Commands::NA => (),
        }
    }
}

fn main() {
    let mut to_recieve = false;
    let mut current_command = Commands::NA;

    let mut request = Request { str: "".to_owned() };

    create_dir_all("./files").unwrap();

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
        let file = deserialize::<Request, Error>(access::<ArchivedRequest, Error>(&buf).expect("corrupted file")).expect("corrupted file");
        println!("{file:?}");

        if to_recieve {
            current_command.execute(&file,&request);
            to_recieve = false;
        }
        if let Some(command) = Commands::from_str(&file.str) {
            request = file;
            to_recieve = true;
            current_command = command;
        }        
    }
}