//! Project-specific errors and From implementations to wrap errors from foreign modules.

use serde_json;
use std::io;
use hyper;
use std::sync::PoisonError;
use regex;
use url;
use std::net;

/// Project-specfic error enum.
#[derive(Debug)]
pub enum Error {
    PoisonedLock,
    JsonEncodingError(serde_json::Error),
    IOError(io::Error),
    RegexError(regex::Error),
    UrlParseError(url::ParseError),
    UnforeseenError,
    ServerError(hyper::Error),
    AddressParsingErr(net::AddrParseError),
    FromUtf8Error,
    HyperError,
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Error::PoisonedLock
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::JsonEncodingError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Error::RegexError(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::UrlParseError(e)
    }
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Error::ServerError(e)
    }
}

impl From<net::AddrParseError> for Error {
    fn from(e: net::AddrParseError) -> Self {
        Error::AddressParsingErr(e)
    }
}
