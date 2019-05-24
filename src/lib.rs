use tiny_http::{Method, Request};

use std::io::Read;

pub const STORE_PREFIX: &str = "/store/";

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

    /// Get url prefix
    fn get_prefix(&self) -> &'static str;

    /// Get a key from store
    fn get(&self, key: String) -> Self::Response;

    /// Save/Update key in store
    fn save(&self, key: String, value: impl Read) -> Self::Response;

    /// Remove a key from store
    fn delete(&self, key: String) -> Self::Response;

    /// Dispatch a request to respective handler methods
    fn dispatch(&self, mut req: Request) {
        let key = get_key(req.url(), self.get_prefix());

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
