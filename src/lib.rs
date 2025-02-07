pub mod crypto;

use std::{io::{Read, Write}, net::TcpStream, path::{Path, PathBuf}};

use rkyv::{access, api::high::{HighDeserializer, HighSerializer}, deserialize, rancor, ser::allocator::ArenaHandle, to_bytes, util::AlignedVec, Archive, Deserialize, Portable, Serialize};

pub const PORT: u16 = 4949;

#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub enum Request {
    Write {
        path: String,
        len: u64,
    },
    MkDir {
        path: String,
    },
    Read {
        path: String,
    },
    EnumDir {
        path: String,
    },
    Delete {
        path: String,
    }
}

#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub struct FileRead {
    pub len: u64,
}

#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub struct DirEnum {
    pub files: Vec<(String, bool)>,
}

pub const PATH: &str = "./files/";
/// doesn't allow symlinks
pub fn sanitize_path(path: &str) -> Option<PathBuf> {
    if !path.is_empty() && (path.starts_with('/') || path.starts_with('\\')) {
        return None;
    }
    if Path::new(path).iter().any(|x| x == ".." || x == ".") || Path::new(path).is_absolute() {
        return None;
    }
    let mut result = PathBuf::from(PATH).canonicalize().unwrap(); // shouldn't panic (hardcoded)
    result.push(path);
    if result.is_symlink() {
        return None;
    }
    Some(result)
}
pub fn sanitize_path_enum(path: &str) -> Option<PathBuf> {
    if path == "." {
        return Some(PathBuf::from(PATH));
    }
    sanitize_path(path)
}

pub struct StructStream<'a> {
    pub inner: &'a mut TcpStream
}
impl<'a> StructStream<'a> {
    pub fn new(stream: &'a mut TcpStream) -> Self {
        Self { inner: stream }
    }
    pub fn write_u64<E: rancor::Source>(&mut self, x: u64) -> Result<(), E> {
        self.inner.write_all(&x.to_le_bytes()).map_err(|x| E::new(x))?;
        Ok(())
    }
    pub fn write_struct<E: rancor::Source>(&mut self, x: &impl for<'b> Serialize<HighSerializer<AlignedVec, ArenaHandle<'b>, E>>) -> Result<(), E> {
        let bytes = to_bytes(x)?;
        self.write_u64::<E>(bytes.len() as u64)?;
        self.inner.write_all(&bytes).map_err(|x| E::new(x))?;
        Ok(())
    }
    pub fn write_buffer<E: rancor::Source>(&mut self, buffer: &[u8]) -> Result<(), E> {
        self.inner.write_all(buffer).map_err(|x| E::new(x))?;
        Ok(())
    }
    pub fn receive_u64<E: rancor::Source>(&mut self) -> Result<u64, E> {
        let mut bytes = [0; 8];
        self.inner.read_exact(&mut bytes).map_err(|x| E::new(x))?;
        Ok(u64::from_le_bytes(bytes))
    }
    #[allow(clippy::uninit_vec)]
    pub fn receive_struct<T: Archive, A: Portable + Deserialize<T, HighDeserializer<E>> + for<'b> rkyv::bytecheck::CheckBytes<rkyv::rancor::Strategy<rkyv::validation::Validator<rkyv::validation::archive::ArchiveValidator<'b>, rkyv::validation::shared::SharedValidator>, E>>, E: rancor::Source>(&mut self) -> Result<T, E> {
        let len = self.receive_u64::<E>()?;
        let mut bytes: Vec<u8> = Vec::with_capacity(len as usize);
        unsafe { bytes.set_len(len as usize) };
        self.inner.read_exact(&mut bytes).map_err(|x| E::new(x))?;
        let val = deserialize(access::<A, E>(&bytes)?)?;
        Ok(val)
    }
    #[allow(clippy::uninit_vec)]
    pub fn receive_buffer<E: rancor::Source>(&mut self, len: u64) -> Result<Vec<u8>, E> {
        let mut bytes: Vec<u8> = Vec::with_capacity(len as usize);
        unsafe { bytes.set_len(len as usize) };
        self.inner.read_exact(&mut bytes).map_err(|x| E::new(x))?;
        Ok(bytes)
    }
}