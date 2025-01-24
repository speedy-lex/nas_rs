use rkyv::{Archive, Serialize, Deserialize};

pub const PORT: u16 = 4949;

#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub struct Request {
    pub str: String,
}