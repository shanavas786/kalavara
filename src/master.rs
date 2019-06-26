//! # master server
//!
//! Master server stores index (key, url of volume server where the value is
//! stored) in rocksdb. Requests are redirected to curresponding volume server
//! after metadata is updated.
//!
//! to start the server, run
//!
//! ```sh
//! master -p 6000 -d /tmp/kalavadb -v http://volume1:6001 http://volume2:6002
//! ```
//!

use rand::{thread_rng, Rng};
use rocksdb::DB;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
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
    volumes: Arc<RwLock<HashMap<String, u32>>>,
}

/// Types of responses that master generates
enum ResponseKind {
    /// Redirect to volume server, 301
    Redirect(String),

    /// 200
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
        if self.volumes.read().unwrap().is_empty() {
            ResponseKind::Unavailable
        } else {
            let volume_url = self.key_to_volume(&key);
            match self.db.put(key.as_bytes(), volume_url.as_bytes()) {
                Ok(_) => {
                    // increment count in map
                    self.increment_count(&volume_url);
                    ResponseKind::Redirect(format!("{}/{}", volume_url, key))
                }
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
                    // decrement count in map
                    self.decrement_count(&volume_url);
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
        let mut volumes_map = self.volumes.write().unwrap();

        let entry = (*volumes_map).entry(volume);
        match entry {
            Entry::Occupied(_) => ResponseKind::Ok("Skipping duplicate volume server".to_string()),
            Entry::Vacant(e) => {
                e.insert(0);
                ResponseKind::Ok("Volume added".to_string())
            }
        }
    }
}

impl Master {
    pub fn new(db: DB, volumes: Vec<String>) -> Master {
        // Create HashMap from url list
        let mut volume_map = HashMap::<String, u32>::new();

        for url in volumes {
            volume_map.insert(url, 0);
        }

        Master {
            db: Arc::new(db),
            volumes: Arc::new(RwLock::new(volume_map)),
        }
    }

    /// translate key to volume url
    /// volume server is selected based on the number of keys it holds.
    /// Server with lesser number of keys are more likely to get selected.
    fn key_to_volume(&self, _key: &str) -> String {
        let volumes_map = self.volumes.read().unwrap();
        let len = volumes_map.len();
        let mut vlms = Vec::<&String>::with_capacity(len);
        let mut counts = Vec::<f32>::with_capacity(len);

        let mut cumulative_count = 0.0f32;
        let mut max_count = 0;

        for (key, value) in volumes_map.iter() {
            vlms.push(key);

            let count = if *value == 0 { 1 } else { *value };

            if count > max_count {
                max_count = count;
            }

            cumulative_count += count as f32;
            counts.push(cumulative_count);
        }

        // invert-normalize counts
        for count in counts.iter_mut() {
            *count = max_count as f32 / *count;
        }

        let mut rng = thread_rng();
        let random = rng.gen_range(0.0, max_count as f32);

        for indx in 0..len {
            if random <= counts[indx] {
                return (*vlms[indx]).to_string();
            }
        }

        vlms[len - 1].to_string()
    }

    /// increment counter for url
    fn increment_count(&self, url: &str) {
        let mut volumes_map = self.volumes.write().unwrap();

        if let Some(count) = (*volumes_map).get_mut(url) {
            *count += 1;
        }
    }

    /// decrement counter for url
    fn decrement_count(&self, url: &str) {
        let mut volumes_map = self.volumes.write().unwrap();
        if let Some(count) = (*volumes_map).get_mut(url) {
            *count -= 1;
        }
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

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_master_crud() {
        let data_dir = tempdir().unwrap();

        let db = match DB::open_default(data_dir) {
            Ok(db) => db,
            Err(e) => panic!("failed to open database: {:?}", e),
        };

        let master = Master::new(
            db,
            vec![
                "server1".to_owned(),
                "server2".to_owned(),
                "server3".to_owned(),
                "server4".to_owned(),
                "server5".to_owned(),
            ],
        );
        let key = "key".to_owned();
        let val = "val".to_owned();

        assert!(match master.get(key.clone()) {
            ResponseKind::NotFound => true,
            _ => false,
        });

        let mut url = String::new();

        assert!(match master.save(key.clone(), val.clone().as_bytes()) {
            ResponseKind::Redirect(to) => {
                url = to;
                true
            }
            _ => false,
        });

        // should redirect to the save volume server
        // in which the key got stored
        assert!(match master.get(key.clone()) {
            ResponseKind::Redirect(to) => to == url,
            _ => false,
        });

        assert_eq!(master.volumes.read().unwrap().get(&url[..7]), Some(&1));

        assert!(match master.delete(key.clone()) {
            ResponseKind::Redirect(to) => to == url,
            _ => false,
        });

        assert_eq!(master.volumes.read().unwrap().get(&url[..7]), Some(&0));
    }

    #[test]
    fn test_master_admin() {
        let data_dir = tempdir().unwrap();

        let db = match DB::open_default(data_dir) {
            Ok(db) => db,
            Err(e) => panic!("failed to open database: {:?}", e),
        };

        let master = Master::new(db, vec!["server1".to_owned(), "server2".to_owned()]);

        assert!(match master.add_volume("server3".to_owned()) {
            ResponseKind::Ok(resp) => resp == "Volume added".to_string(),
            _ => false,
        });

        assert_eq!(master.volumes.read().unwrap().len(), 3);

        assert!(match master.add_volume("server3".to_owned()) {
            ResponseKind::Ok(resp) => resp == "Skipping duplicate volume server".to_string(),
            _ => false,
        });

        assert_eq!(master.volumes.read().unwrap().len(), 3);
    }
}
