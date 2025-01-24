use rkyv::{Archive, Serialize, Deserialize};

pub const PORT: u16 = 4949;

#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub enum Request {
    Write {
        path: String,
        len: u64,

    },
    Delete {
        path: String,
    }
}