use std::path::{Path, PathBuf};

use rkyv::{Archive, Serialize, Deserialize};

pub const PORT: u16 = 4949;

#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub enum Request {
    Write {
        path: String,
        len: u64,
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