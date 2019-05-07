use rand::{thread_rng, Rng};
use rocksdb::DB;
use tiny_http::{Header, Method, Request, Response};

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

macro_rules! redirect {
    ($url:expr) => {
        Response::from_string("")
            .with_status_code(307)
            .with_header(Header::from_str($url).unwrap())
    };
}

fn admin_handle(_volumes: &Mutex<Vec<String>>, req: Request) {
    // FIXME
    let _ = req.respond(Response::from_string("admin"));
}

fn store_handler(db: &DB, volumes: &Mutex<Vec<String>>, mut req: Request) {
    // TODO remove query params
    let path = String::from(req.url());

    let mut body = String::new();
    let _ = req.as_reader().read_to_string(&mut body);

    let _ = match *req.method() {
        Method::Get => match db.get(path.as_bytes()) {
            Ok(Some(volume)) => {
                let volume_url = volume.to_utf8().unwrap();
                req.respond(redirect!(&format!("Location:{}{}", volume_url, path)))
            }
            Ok(None) => req.respond(Response::from_string("Key not found").with_status_code(404)),
            Err(_) => req.respond(Response::from_string("Server Error").with_status_code(500)),
        },
        Method::Post | Method::Put => {
            let vlms = volumes.lock().unwrap();

            if vlms.is_empty() {
                req.respond(Response::from_string("No volume servers found").with_status_code(503))
            } else {
                let mut rng = thread_rng();
                let n: usize = rng.gen_range(0, vlms.len());
                let volume = vlms.get(n).unwrap();

                match db.put(path.as_bytes(), volume) {
                    Ok(_) => req.respond(redirect!(&format!("Location:{}{}", volume, path))),
                    Err(_) => {
                        req.respond(Response::from_string("Server Error").with_status_code(500))
                    }
                }
            }
        }
        Method::Delete => {
            match db.get(path.as_bytes()) {
                Ok(Some(volume)) => {
                    let volume_url = volume.to_utf8().unwrap();
                    println!("key found on volume {}", volume_url);

                    // delete it from db
                    db.delete(path.as_bytes()).unwrap();
                    req.respond(redirect!(&format!("Location:{}{}", volume_url, path)))
                }
                Ok(None) => {
                    req.respond(Response::from_string("Key not found").with_status_code(404))
                }
                Err(_) => req.respond(Response::from_string("Server Error").with_status_code(500)),
            }
        }
        _ => req.respond(Response::from_string("Method not allowed").with_status_code(405)),
    };
}

fn req_handler(db: &DB, volumes: &Mutex<Vec<String>>, req: Request) {
    let path = String::from(req.url());

    if path.starts_with("/store/") {
        store_handler(db, volumes, req);
    } else if path.starts_with("/admin/") {
        admin_handle(volumes, req);
    } else {
        let _ = req.respond(Response::from_string("Invalid Path").with_status_code(404));
    }
}

pub fn start(port: u16, data_dir: &str, volumes: Vec<String>) {
    let db = match DB::open_default(data_dir) {
        Ok(db) => Arc::new(db),
        Err(e) => panic!("failed to open database: {:?}", e),
    };

    // TODO use RWLOck
    let volumes: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(volumes));
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    let server = Arc::new(tiny_http::Server::http(addr).unwrap());
    let mut handles = Vec::new();

    // TODO: defult to numcpus
    for _ in 0..4 {
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
