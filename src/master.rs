use rand::seq::IteratorRandom;
use rand::thread_rng;
use rocksdb::DB;

use std::collections::HashSet;
use std::io::Read;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::thread;
use tiny_http::{Request, Server};

use crate::{Respond, Service, STORE_PREFIX};

/// Master store
struct Master {
    db: Arc<DB>,
    volumes: Arc<RwLock<HashSet<String>>>,
}

/// Types of responses that master generates
enum ResponseKind {
    /// Redirect to volume server, 301
    Redirect(String),

    /// Key not found, 404
    NotFound,

    /// Error occured, 500
    ServerError,

    /// Method not allowed, 405
    NotAllowed,

    /// Unavailable, 503
    Unavailable,
}

impl Default for ResponseKind {
    fn default() -> Self {
        ResponseKind::NotAllowed
    }
}

impl Respond for ResponseKind {
    fn respond(self, req: Request) {
        use ResponseKind::*;

        let _ = match self {
            Redirect(url) => req.respond(redirect!(&format!("Location:{}", url))),
            NotFound => req.respond(resp!("Key not found", 404)),
            ServerError => req.respond(resp!("Server error", 500)),
            NotAllowed => req.respond(resp!("Method not allowd", 405)),
            Unavailable => req.respond(resp!("Service unavailable", 503)),
        };
    }
}

impl Master {
    pub fn new(db: DB, volumes: Vec<String>) -> Master {
        Master {
            db: Arc::new(db),
            volumes: Arc::new(RwLock::new(volumes.into_iter().collect())),
        }
    }

    /// translate key to volume url
    /// TODO: more intelligent selection
    fn key_to_volume(&self, _key: &str) -> String {
        let mut rng = thread_rng();
        let mut volume = None;
        let volumes = self.volumes.read().unwrap();

        while volume.is_none() {
            volume = volumes.iter().choose(&mut rng);
        }

        volume.unwrap().to_owned()
    }
}

impl Service for Master {
    type Response = ResponseKind;

    fn get_prefix(&self) -> &'static str {
        STORE_PREFIX
    }

    fn get(&self, key: String) -> Self::Response {
        match self.db.get(key.as_bytes()) {
            Ok(Some(volume)) => {
                ResponseKind::Redirect(format!("{}/{}", volume.to_utf8().unwrap().to_string(), key))
            }
            Ok(None) => ResponseKind::NotFound,
            Err(_) => ResponseKind::ServerError,
        }
    }

    fn save(&self, key: String, _value: impl Read) -> Self::Response {
        let vlms = self.volumes.read().unwrap();

        if vlms.is_empty() {
            ResponseKind::Unavailable
        } else {
            let volume = self.key_to_volume(&key);
            match self.db.put(key.as_bytes(), volume.as_bytes()) {
                Ok(_) => ResponseKind::Redirect(format!("{}/{}", volume, key)),
                Err(_) => ResponseKind::ServerError,
            }
        }
    }

    fn delete(&self, key: String) -> Self::Response {
        match self.db.get(key.as_bytes()) {
            Ok(Some(volume)) => {
                let volume_url = volume.to_utf8().unwrap();

                // delete it from db
                if self.db.delete(key.as_bytes()).is_ok() {
                    ResponseKind::Redirect(format!("{}/{}", volume_url, key))
                } else {
                    ResponseKind::ServerError
                }
            }
            Ok(None) => ResponseKind::NotFound,
            Err(_) => ResponseKind::ServerError,
        }
    }

    // TODO: match store prefix
}

/// starts a kalavara master server
/// # Arguments
///
/// * `port` - Port name to listen at
/// * `data_dir` - Database directory
/// * `threads` - Number of threads to spawn
/// * `volumes` - List of volume servers
///
pub fn start(port: u16, data_dir: &str, threads: u16, volumes: Vec<String>) {
    let db = match DB::open_default(data_dir) {
        Ok(db) => db,
        Err(e) => panic!("failed to open database: {:?}", e),
    };

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    let server = match Server::http(addr) {
        Ok(server) => Arc::new(server),
        Err(e) => panic!("failed to start http server: {:?}", e),
    };

    let master = Arc::new(Master::new(db, volumes));

    let mut handles = Vec::new();

    for _ in 0..threads {
        let server = server.clone();
        let handler = master.clone();

        handles.push(thread::spawn(move || {
            for rq in server.incoming_requests() {
                handler.dispatch(rq);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
