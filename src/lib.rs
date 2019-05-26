//! A distributed persistent key value store that speaks http. Inspired by
//! [minkeyvalue](https://github.com/geohot/minikeyvalue).
//!
//! ## Usage
//!
//! 1. insert a key-value
//!
//! ```sh
//! curl -XPUT -L -d value http://localhost:6000/store/key
//! ```
//!
//! 2. retrive value
//!
//! ```sh
//! curl -XGET -L http://localhost:6000/store/key
//! ```
//!
//! 3. delete a key
//!
//! ```sh
//! curl -XDELETE -L http://localhost:6000/store/key
//! ```
//!
//! 4. register a new volume server with master
//!
//! ```sh
//! curl -XPOST -d "http://newvolume.server" http://localhost:6000/admin/add-volume
//! ```

use tiny_http::{Method, Request};

use std::io::Read;

const STORE_PREFIX: &str = "/store/";
const ADMIN_PREFIX: &str = "/admin/";

/// returns the key from url string by removing /store/ prefix and query params if any
fn get_key(url: &str, prefix: &str) -> String {
    let pfx_len = if url.starts_with(prefix) {
        prefix.len()
    } else {
        0
    };

    // remove query params if any
    match url.find('?') {
        None => String::from(&url[pfx_len..]),
        Some(indx) => String::from(&url[pfx_len..indx]),
    }
}

#[test]
fn test_get_key() {
    let url = "/store/originalkey?q=this&that=that#foo";
    assert_eq!(get_key(url, "/store/"), String::from("originalkey"));
}

/// Trait that send http response to a request
/// ResponseKind Should implement this
trait Respond {
    fn respond(self, req: Request);
}

/// Kalavara Store service trait
/// Defines methods that stores needs to implement
trait Service: Sync + Send {
    /// ResponseTye
    /// Should know how to respond to a request
    type Response: Respond + Default;

    /// Get a key from store
    fn get(&self, key: String) -> Self::Response;

    /// Save/Update key in store
    fn save(&self, key: String, value: impl Read) -> Self::Response;

    /// Remove a key from store
    fn delete(&self, key: String) -> Self::Response;

    /// Dispatch a request to respective handler methods
    fn dispatch(&self, mut req: Request) {
        let key = get_key(req.url(), STORE_PREFIX);

        let resp = match *req.method() {
            Method::Get => self.get(key),
            Method::Post | Method::Put => self.save(key, req.as_reader()),
            Method::Delete => self.delete(key),
            _ => Default::default(),
        };

        resp.respond(req);
    }
}

#[macro_use]
mod macros;
pub mod master;
pub mod volume;
