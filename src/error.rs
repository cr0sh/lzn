use bincode::Error as BincodeError;
use sled::Error as SledError;
use std::io::Error as IOError;
use zip::result::ZipError;

use err_derive::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "Bincode serialization/deserialization failure")]
    Bincode(#[error(source)] BincodeError),
    #[error(display = "zip-rs library failure")]
    Zip(#[error(source)] ZipError),
    #[error(display = "Sled database failure")]
    Sled(#[error(source)] SledError),
    #[error(display = "std::io failure")]
    IO(#[error(source)] IOError),
}

pub type Result<T> = std::result::Result<T, Error>;
