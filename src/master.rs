use rand::seq::IteratorRandom;
use rand::thread_rng;
use rocksdb::DB;

use std::collections::HashSet;
use std::io::Read;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::thread;
use tiny_http::{Method, Request, Server};

use crate::get_key;
use crate::{Respond, Service, ADMIN_PREFIX, STORE_PREFIX};

/// Master store
struct Master {
    db: Arc<DB>,
    volumes: Arc<RwLock<HashSet<String>>>,
}

/// Types of responses that master generates
enum ResponseKind {
    /// Redirect to volume server, 301
    Redirect(String),

    Ok(String),

    /// Key not found, 404
    NotFound,

    /// Error occured, 500
    ServerError,

    /// Method not allowed, 405
    NotAllowed,

    /// Unavailable, 503
    Unavailable,
}

/// Admin service interfaces
trait AdminService: Sync + Send {
    /// add new volume server
    fn add_volume(&self, url: String) -> ResponseKind;

    /// dispatch request to admin service
    fn dispatch(&self, mut req: Request) {
        let path = get_key(req.url(), ADMIN_PREFIX);

        let resp = match (path.as_str(), req.method()) {
            ("add-volume", &Method::Post) => {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);

                self.add_volume(body)
            }
            ("add-volume", _) => ResponseKind::NotAllowed,
            (_, _) => ResponseKind::NotFound,
        };

        resp.respond(req);
    }
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
            Ok(txt) => req.respond(resp!(txt, 200)),
            NotFound => req.respond(resp!("Key not found", 404)),
            ServerError => req.respond(resp!("Server error", 500)),
            NotAllowed => req.respond(resp!("Method not allowd", 405)),
            Unavailable => req.respond(resp!("Service unavailable", 503)),
        };
    }
}

impl Service for Master {
    type Response = ResponseKind;

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
}

impl AdminService for Master {
    fn add_volume(&self, volume: String) -> ResponseKind {
        if self.volumes.write().unwrap().insert(volume) {
            ResponseKind::Ok("Volume added".to_string())
        } else {
            ResponseKind::Ok("Skipping duplicate volume".to_string())
        }
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

    fn dispatch(&self, req: Request) {
        let url = req.url();

        if url.starts_with(STORE_PREFIX) {
            Service::dispatch(self, req);
        } else if url.starts_with(ADMIN_PREFIX) {
            AdminService::dispatch(self, req);
        } else {
            let _ = req.respond(resp!("Path not found", 404));
        }
    }
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
