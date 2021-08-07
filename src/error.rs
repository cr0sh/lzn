use bincode::Error as BincodeError;
use diesel::result::Error as DieselError;
use serde_json::Error as JSONError;
use std::io::Error as IOError;
use std::num::ParseIntError;
use ureq::Error as UreqError;
use zip::result::ZipError;

use err_derive::Error;

// TODO: Rename this to ErrorKind and use Error struct, consisting context string and ErrorKind
#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "Bincode serialization/deserialization failure")]
    Bincode(#[error(source)] BincodeError),
    #[error(display = "zip-rs library failure")]
    Zip(#[error(source)] ZipError),
    #[error(display = "std::io failure")]
    IO(#[error(source)] IOError),
    #[error(display = "Cannot parse number")]
    ParseInt(#[error(source)] ParseIntError),
    #[error(display = "Diesel failure")]
    Diesel(#[error(source)] DieselError),
    #[error(display = "Reqwest failure")]
    AuthFailure,
    #[error(display = "{}", _0)]
    StaticStr(&'static str),
    #[error(display = "JSON Serialization/Deserialization failure")]
    Serde(#[error(source)] JSONError),
    #[error(display = "Currently unavailable episode")]
    UnavailableEpisode,
    #[error(display = "ureq failure")]
    Ureq(#[error(source)] Box<UreqError>),
}

impl From<&'static str> for Error {
    fn from(s: &'static str) -> Self {
        Self::StaticStr(s)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
