//! Defines a Bins trait for storing Requests as well as a default in-memory implementation
//! of that trait for easy testing.
use std::collections::HashMap;

use models::*;

/// ADT for denoting status when inserting a request with a bin id.
pub enum InsertRequestStatus {
    /// Insert successful
    Ok,
    /// Insert failed because no bin by that Id exists
    NoSuchBin,
}

/// ADT For deleting a bin by id
pub enum DeleteBinStatus {
    /// Successfully deleted
    Ok,
    /// No such bin. Deletion was not carried out.
    NoSuchBin,
}

/// A Bin holds a bunch of requests. For now it's just an alias for a Vector.
pub type Bin = Vec<Request>;

/// Trait for storage operations for Requests.
pub trait Bins {
    /// Returns a BinSummary of a newly-reated bin. The Id in the summary
    /// must be unique at the time of creation.
    fn create_bin(&mut self) -> BinSummary;

    /// Delete a bin by Id
    fn delete_bin(&mut self, id: &Id) -> DeleteBinStatus;

    /// Get a bin (not just a summary) by Id
    fn get_bin<'a>(&'a self, id: &'a Id) -> Option<&Bin>;

    /// Get a bin summary by Id
    fn get_bin_summary(&self, id: &Id) -> Option<BinSummary>;

    /// Get bin summaries for all currently-stored bins
    fn get_bin_summaries(&self) -> HashMap<Id, BinSummary>;

    /// Insert a request into a Bin using a bin Id.
    fn insert_request(&mut self, id: &Id, request: Request) -> InsertRequestStatus;
}

/// A simple in-memory implementation of Bins.
#[derive(Debug)]
pub struct InMemoryBins {
    pub bins: HashMap<Id, Vec<Request>>,
}

impl InMemoryBins {
    pub fn new() -> InMemoryBins {
        InMemoryBins { bins: HashMap::new() }
    }
}

impl Bins for InMemoryBins {
    fn create_bin(&mut self) -> BinSummary {
        let mut uuid = Id::random();
        while self.bins.contains_key(&uuid) {
            uuid = Id::random();
        }
        self.bins.insert(uuid.to_owned(), Vec::new());
        BinSummary {
            id: uuid,
            request_count: 0,
        }
    }

    fn delete_bin(&mut self, id: &Id) -> DeleteBinStatus {
        match self.bins.remove(id) {
            Some(_) => DeleteBinStatus::Ok,
            _ => DeleteBinStatus::NoSuchBin,
        }
    }

    fn get_bin_summary(&self, id: &Id) -> Option<BinSummary> {
        self.bins.get(id).map(|b| {
            BinSummary {
                id: id.to_owned(),
                request_count: b.len(),
            }
        })
    }

    fn get_bin_summaries(&self) -> HashMap<Id, BinSummary> {
        let mut map: HashMap<Id, BinSummary> = HashMap::new();
        for (k, b) in self.bins.iter() {
            map.insert(
                k.to_owned(),
                BinSummary {
                    id: k.to_owned(),
                    request_count: b.len(),
                },
            );
        }
        map
    }

    fn get_bin(&self, id: &Id) -> Option<&Bin> {
        self.bins.get(id)
    }

    fn insert_request(&mut self, id: &Id, request: Request) -> InsertRequestStatus {
        match self.bins.get_mut(id) {
            Some(bin) => {
                bin.push(request);
                InsertRequestStatus::Ok
            }
            None => InsertRequestStatus::NoSuchBin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_inmemory_bin_creation() {
        let mut bins = InMemoryBins::new();
        let _ = bins.create_bin();
    }

    #[test]
    fn test_inmemory_bin_deletion() {
        let mut bins = InMemoryBins::new();
        let bin = bins.create_bin();
        bins.delete_bin(&bin.id);
        assert!(bins.bins.is_empty())
    }

    #[test]
    fn test_inmemory_get_bin_summary() {
        let mut bins = InMemoryBins::new();
        let bin = bins.create_bin();
        let req = Request {
            content_length: None,
            content_type: Some("fake".to_owned()),
            time: 123,
            method: "GET".to_owned(),
            path: "/whoa".to_owned(),
            body: None,
            headers: HashMap::new(),
            query_string: HashMap::new(),
        };
        bins.insert_request(&bin.id, req);

        let summary = bins.get_bin_summary(&bin.id).unwrap();
        assert_eq!(summary.request_count, 1)
    }

    #[test]
    fn test_inmemory_get_bin() {
        let mut bins = InMemoryBins::new();
        let bin = bins.create_bin();
        let req = Request {
            content_length: None,
            content_type: Some("fake".to_owned()),
            time: 123,
            method: "GET".to_owned(),
            path: "/whoa".to_owned(),
            body: None,
            headers: HashMap::new(),
            query_string: HashMap::new(),
        };
        bins.insert_request(&bin.id, req);

        let summary = bins.get_bin(&bin.id).unwrap();
        // Don't want to implement Clone in case we clone the requests by accident somewhere..
        assert_eq!(
            summary[0],
            Request {
                content_length: None,
                content_type: Some("fake".to_owned()),
                time: 123,
                method: "GET".to_owned(),
                path: "/whoa".to_owned(),
                body: None,
                headers: HashMap::new(),
                query_string: HashMap::new(),
            }
        )
    }

    #[test]
    fn test_inmemory_get_bin_summaries() {
        let mut bins = InMemoryBins::new();
        let bin = bins.create_bin();
        let req = Request {
            content_length: None,
            content_type: Some("fake".to_owned()),
            time: 123,
            method: "GET".to_owned(),
            path: "/whoa".to_owned(),
            body: None,
            headers: HashMap::new(),
            query_string: HashMap::new(),
        };
        bins.insert_request(&bin.id, req);

        let summaries = bins.get_bin_summaries();
        assert_eq!(summaries.get(&bin.id).unwrap().request_count, 1)
    }
}
