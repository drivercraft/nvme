#[derive(Debug, Clone, Copy)]
pub enum Error {
    NoMemory,
    Layout,
}

pub type Result<T = ()> = core::result::Result<T, Error>;
