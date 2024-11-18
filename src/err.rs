#[derive(Debug, Clone, Copy)]
pub enum Error {
    NoMemory,
    Layout,
    Unknown(&'static str),
}

pub type Result<T = ()> = core::result::Result<T, Error>;
