//! Project-specific errors and From implementations to wrap errors from foreign modules.

use rustc_serialize::json;
use std::io;
use hyper;
use std::sync::PoisonError;
use regex;
use url;

/// Project-specfic error enum.
#[derive(Debug)]
pub enum Error {
    PoisonedLock,
    JsonEncodingError(json::EncoderError),
    IOError(io::Error),
    RegexError(regex::Error),
    UrlParseError(url::ParseError),
    UnforeseenError,
    ServerError(hyper::Error),
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Error::PoisonedLock
    }
}

impl From<json::EncoderError> for Error {
    fn from(e: json::EncoderError) -> Self {
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
