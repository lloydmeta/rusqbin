//! Various models and associated helpers and type aliases.
//!
//! The models here use String instead of &str
//!
//!   1. The Json serdes traits don't work well with &str (specifically decoding),
//!      and while we can use fascade/presentation models, that adds more code and more
//!      run-time conversion.
//!   2. The fact that these models need to go over threads (because they need to be used
//!      in the http server) make it annoying to deal with lifecycles (cloning might be
//!      an answer).
//!   3. String is easier to just store somewhere because you don't need to worry about
//!      lifecycle annotations
//!
//! Having said that, it would be nice to have them use &str instead of String, if possible (^_^ "

use std::collections::HashMap;

use std::fmt;
use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};
use uuid::Uuid;

use regex::Regex;

const ID_REGEXP: &'static str =
    r"^((?i)[A-F0-9]{8}\-[A-F0-9]{4}\-4[A-F0-9]{3}\-[89AB][A-F0-9]{3}\-[A-F0-9]{12})$";

/// Id type for Request Bins. Wraps a String.
///
/// JSON encodes to just a plain ol' String (as opposed to an object).
///
/// To construct an Id, use an IdExtractor's parse method, or use the static `random()` method.
///
/// ```
/// # use rusqbin::models::*;
/// let id = Id::random();
///
/// let id_extractor = IdExtractor::new();
///
/// let parsed_id = {
///   let id_as_str = id.value();
///   id_extractor.parse(id_as_str)
/// };
///
/// assert_eq!(parsed_id, Some(id));
///
/// let nope = id_extractor.parse("lulz");
/// assert_eq!(nope, None);
/// ```
#[derive(PartialEq, Debug, Eq, Hash, Clone)]
pub struct Id(String);

impl Id {
    /// Generates a random Id
    pub fn random() -> Id {
        Id(Uuid::new_v4().to_string())
    }

    /// Gets the value out of an Id.
    pub fn value(&self) -> &str {
        &*self.0
    }
}

/// Parses normal Strings into an Id according to the compiled Regex that it
/// holds.
///
/// Exists purely because we can't have Regexp constants in Rust (yet).
///
/// For usage, see docs for Id.
pub struct IdExtractor(Regex);

impl IdExtractor {
    /// Returns a new Id extractor
    /// ```
    /// use rusqbin::models::*;
    ///
    /// let id_extractor = IdExtractor::new();
    /// assert!(id_extractor.parse("hello").is_none()); // does not fit our id pattern
    /// ```
    pub fn new() -> IdExtractor {
        match Regex::new(ID_REGEXP) {
            Ok(regex) => IdExtractor(regex),
            _ => unreachable!(), // yo the regexp is perfect.
        }
    }

    /// Parses a string into an Id if it is of the right format.
    pub fn parse(&self, s: &str) -> Option<Id> {
        let caps = self.0.captures(s);
        caps.and_then(|c| c.get(1).map(|r| Id(r.as_str().to_owned())))
    }
}

impl Encodable for Id {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_str(&*self.0)
    }
}

impl Decodable for Id {
    fn decode<S: Decoder>(s: &mut S) -> Result<Id, S::Error> {
        s.read_str().map(|s| Id(s.to_owned()))
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A record of an HTTP request made to the server.
#[derive(PartialEq, Debug, Eq, RustcDecodable, RustcEncodable)]
pub struct Request {
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    pub time: i64,
    pub method: String,
    pub path: String,
    pub body: Option<String>,
    pub headers: HashMap<String, Vec<String>>,
    pub query_string: HashMap<String, Vec<String>>,
}

/// Summary of a Bin of requests.
#[derive(PartialEq, Debug, Eq, RustcDecodable, RustcEncodable)]
pub struct BinSummary {
    pub id: Id,
    pub request_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_serialize::json;

    #[test]
    fn test_idextractor_instantiation() {
        let _ = IdExtractor::new();
    }

    #[test]
    fn test_id_json_encoding_decoding() {
        let id = Id::random();
        let encoded = json::encode(&id).unwrap();
        let decoded: Id = json::decode(&*encoded).unwrap();
        assert_eq!(decoded, id);
        assert_eq!(format!("\"{}\"", id), encoded); // should be a raw JSON string, not wrapped in an object
    }
}
