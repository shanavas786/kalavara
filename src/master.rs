use rand::seq::IteratorRandom;
use rand::thread_rng;
use rocksdb::DB;
use tiny_http::{Method, Request};

use std::collections::HashSet;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::thread;

const STORE_PREFIX: &str = "/store/";
const ADMIN_PREFIX: &str = "/admin/";

/// returns the key from url string by removing /store/ prefix and query params if any
fn get_key(url: &str) -> String {
    let pfx_len = STORE_PREFIX.len();

    // remove query params if any
    match url.find('?') {
        None => String::from(&url[pfx_len..]),
        Some(indx) => String::from(&url[pfx_len..indx]),
    }
}

#[test]
fn test_get_key() {
    let url = "/store/originalkey?q=this&that=that#foo";
    assert_eq!(get_key(url), String::from("originalkey"));
}

fn admin_handle(_volumes: &RwLock<HashSet<String>>, req: Request) {
    // FIXME
    let _ = req.respond(resp!("admin"));
}

fn store_handler(db: &DB, volumes: &RwLock<HashSet<String>>, mut req: Request) {
    let key = get_key(req.url());

    let mut body = String::new();
    let _ = req.as_reader().read_to_string(&mut body);

    let _ = match *req.method() {
        Method::Get => match db.get(key.as_bytes()) {
            Ok(Some(volume)) => {
                let volume_url = volume.to_utf8().unwrap();
                req.respond(redirect!(&format!("Location:{}/{}", volume_url, key)))
            }
            Ok(None) => req.respond(resp!("Key not found", 404)),
            Err(_) => req.respond(resp!("Server Error", 500)),
        },
        Method::Post | Method::Put => {
            let vlms = volumes.read().unwrap();

            if vlms.is_empty() {
                req.respond(resp!("No volume servers found", 503))
            } else {
                let mut rng = thread_rng();
                let mut volume = None;
                while volume.is_none() {
                    volume = vlms.iter().choose(&mut rng);
                }

                match db.put(key.as_bytes(), volume.unwrap()) {
                    Ok(_) => {
                        req.respond(redirect!(&format!("Location:{}/{}", volume.unwrap(), key)))
                    }
                    Err(_) => req.respond(resp!("Server Error", 500)),
                }
            }
        }
        Method::Delete => {
            match db.get(key.as_bytes()) {
                Ok(Some(volume)) => {
                    let volume_url = volume.to_utf8().unwrap();

                    // delete it from db
                    db.delete(key.as_bytes()).unwrap();
                    req.respond(redirect!(&format!("Location:{}/{}", volume_url, key)))
                }
                Ok(None) => req.respond(resp!("Key not found", 404)),
                Err(_) => req.respond(resp!("Server Error", 500)),
            }
        }
        _ => req.respond(resp!("Method not allowed", 405)),
    };
}

fn req_handler(db: &DB, volumes: &RwLock<HashSet<String>>, req: Request) {
    let path = String::from(req.url());

    if path.starts_with(STORE_PREFIX) {
        store_handler(db, volumes, req);
    } else if path.starts_with(ADMIN_PREFIX) {
        admin_handle(volumes, req);
    } else {
        let _ = req.respond(resp!("Invalid Path", 404));
    }
}

pub fn start(port: u16, data_dir: &str, threads: u16, volumes: Vec<String>) {
    let db = match DB::open_default(data_dir) {
        Ok(db) => Arc::new(db),
        Err(e) => panic!("failed to open database: {:?}", e),
    };

    let volumes: Arc<RwLock<HashSet<String>>> =
        Arc::new(RwLock::new(volumes.into_iter().collect()));
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    let server = Arc::new(tiny_http::Server::http(addr).unwrap());
    let mut handles = Vec::new();

    for _ in 0..threads {
        let server = server.clone();
        let db = db.clone();
        let volumes = volumes.clone();

        handles.push(thread::spawn(move || {
            for rq in server.incoming_requests() {
                req_handler(&db, &volumes, rq);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
