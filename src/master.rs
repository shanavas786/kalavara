use rocksdb::DB;
use tiny_http::{Header, Method, Request, Response};

use std::io::Cursor;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

type KResponse = Response<Cursor<Vec<u8>>>;


fn admin_handle(volumes: &Mutex<Vec<String>>, req: &mut Request) -> KResponse {
    // FIXME
    Response::from_string("admin")
}

fn store_handler(db: &DB, volumes: &Mutex<Vec<String>>, req: &mut Request) -> KResponse {
    // TODO remove query params
    let path = String::from(req.url());

    let mut body = String::new();
    let _ = req.as_reader().read_to_string(&mut body);

    match req.method() {
        &Method::Get => {
            match db.get(path.as_bytes()) {
                Ok(Some(volume)) => {
                    let volume_url = volume.to_utf8().unwrap();
                    Response::from_string("").with_status_code(302).with_header(
                        Header::from_str(&format!("Location:{}{}", volume_url, path)).unwrap(),
                    )
                }
                Ok(None) => Response::from_string("Key not found").with_status_code(404),
                Err(_) => Response::from_string("Server Error").with_status_code(500),
            }
        }
        &Method::Post => {
            let vlms = volumes.lock().unwrap();

            if vlms.is_empty() {
                Response::from_string("No volume servers found").with_status_code(503)
            } else {
                // FIXME get random nubmer
                let volume = vlms.get(0).unwrap();

                match db.put(path.as_bytes(), volume) {
                    Ok(_) => Response::from_string("").with_status_code(307).with_header(
                        Header::from_str(&format!("Location:{}{}", volume, path)).unwrap(),
                    ),
                    Err(_) => Response::from_string("Server Error").with_status_code(500),
                }
            }
        }
        &Method::Delete => {
            match db.get(path.as_bytes()) {
                Ok(Some(volume)) => {
                    let volume_url = volume.to_utf8().unwrap();
                    println!("key found on volume {}", volume_url);

                    // delete it from db
                    let _ = db.delete(path.as_bytes()).unwrap();

                    Response::from_string("").with_status_code(302).with_header(
                        Header::from_str(&format!("Location:{}{}", volume_url, path)).unwrap(),
                    )
                }
                Ok(None) => Response::from_string("Key not found").with_status_code(404),
                Err(_) => Response::from_string("Server Error").with_status_code(500),
            }
        }
        _ => Response::from_string("Method not allowed").with_status_code(405),
    }
}

fn req_handler(
    db: &DB,
    volumes: &Mutex<Vec<String>>,
    req: &mut Request,
) -> Response<Cursor<Vec<u8>>> {
    let path = String::from(req.url());

    if path.starts_with("/store/") {
        store_handler(db, volumes, req)
    } else if path.starts_with("/admin/") {
        admin_handle(volumes, req)
    } else {
        Response::from_string("Invalid Path").with_status_code(404)
    }
}

pub fn start(port: u16, data_dir: &str) {
    let db = match DB::open_default(data_dir) {
        Ok(db) => Arc::new(db),
        Err(e) => panic!("failed to open database: {:?}", e),
    };

    // TODO use RWLOck
    // FIXME remove default volume server
    let volumes: Arc<Mutex<Vec<String>>> =
        Arc::new(Mutex::new(vec!["http://localhost:7000".to_owned()]));
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    let server = Arc::new(tiny_http::Server::http(addr).unwrap());
    let mut handles = Vec::new();

    // TODO: defult to numcpus
    for _ in 0..4 {
        let server = server.clone();
        let db = db.clone();
        let volumes = volumes.clone();

        handles.push(thread::spawn(move || {
            for mut rq in server.incoming_requests() {
                let response = req_handler(&db, &volumes, &mut rq);
                let _ = rq.respond(response);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
