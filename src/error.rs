use bincode::Error as BincodeError;
use diesel::result::Error as DieselError;
use std::io::Error as IOError;
use std::num::ParseIntError;

#[cfg(feature = "migrate")]
use zip::result::ZipError;

use err_derive::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "Bincode serialization/deserialization failure")]
    Bincode(#[error(source)] BincodeError),
    #[cfg(feature = "migrate")]
    #[error(display = "zip-rs library failure")]
    Zip(#[error(source)] ZipError),
    #[error(display = "std::io failure")]
    IO(#[error(source)] IOError),
    #[error(display = "Cannot parse number")]
    ParseInt(#[error(source)] ParseIntError),
    #[error(display = "Diesel failure")]
    Diesel(#[error(source)] DieselError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
