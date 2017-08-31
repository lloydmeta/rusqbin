//! Project-specific errors and From implementations to wrap errors from foreign modules.

use serde_json;
use std::io;
use hyper;
use std::sync::PoisonError;
use regex;
use url;
use std::net;

use std::error::Error as StdErr;
use std::fmt;
use std::fmt::Display;

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


impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        use self::Error::*;
        match self {
            &PoisonedLock => write!(f, "Poisoned lock error"),
            &UnforeseenError => write!(f, "Unforeseen error"),
            &FromUtf8Error => write!(f, "From UTF8 error"),
            &HyperError => write!(f, "Hyper error"),
            &AddressParsingErr(ref e) => e.fmt(f),
            &JsonEncodingError(ref e) => e.fmt(f),
            &IOError(ref e) => e.fmt(f),
            &RegexError(ref e) => e.fmt(f),
            &UrlParseError(ref e) => e.fmt(f),
            &ServerError(ref e) => e.fmt(f),
        }
    }
}

impl StdErr for Error {
    fn description(&self) -> &str {
        use self::Error::*;
        match self {
            &PoisonedLock => "Poisoned Lock",
            &UnforeseenError => "Unforeseen Error",
            &FromUtf8Error => "UTF8 Conversion Error",
            &HyperError => "Hyper Error",
            &AddressParsingErr(ref e) => e.description(),
            &JsonEncodingError(ref e) => e.description(),
            &IOError(ref e) => e.description(),
            &RegexError(ref e) => e.description(),
            &UrlParseError(ref e) => e.description(),
            &ServerError(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&StdErr> {
        use self::Error::*;
        match self {
            &JsonEncodingError(ref e) => Some(e),
            &IOError(ref e) => Some(e),
            &RegexError(ref e) => Some(e),
            &UrlParseError(ref e) => Some(e),
            &ServerError(ref e) => Some(e),
            &AddressParsingErr(ref e) => Some(e),
            _ => None,
        }
    }
}
