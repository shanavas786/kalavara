use md5::compute as compute_md5;
use tempfile::NamedTempFile;
use tiny_http::{Method, Request, Response};

use std::fs::{create_dir_all, remove_file, File};
use std::io::{Error, ErrorKind, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

fn req_handler(data_dir: &str, mut req: Request) {
    let path = format!("{:x}", compute_md5(req.url().as_bytes()));
    let mut body = String::new();
    let _ = req.as_reader().read_to_string(&mut body);

    let mut dest_path = PathBuf::from(data_dir);
    dest_path.push(path.get(0..1).unwrap());
    dest_path.push(path.get(1..2).unwrap());
    dest_path.push(path.get(2..).unwrap());

    let _ = match *req.method() {
        Method::Get => {
            let file = File::open(dest_path);

            match file {
                Ok(file) => req.respond(Response::from_file(file)),
                Err(_) => req.respond(Response::from_string("Server Error").with_status_code(500)),
            }
        }
        Method::Post => {
            let tmpdir = Path::new(data_dir).join("tmp");

            match NamedTempFile::new_in(tmpdir) {
                Ok(mut tmpfile) => {
                    match tmpfile
                        .write(body.as_bytes())
                        .and(create_dir_all(dest_path.parent().unwrap()))
                        .and(
                            tmpfile
                                .persist(dest_path)
                                .map_err(|_| Error::new(ErrorKind::Other, "")),
                        ) {
                        Ok(_) => {
                            req.respond(Response::from_string("Inserted").with_status_code(201))
                        }
                        _ => req.respond(
                            Response::from_string("Unable to create file").with_status_code(500),
                        ),
                    }
                }
                Err(_) => req
                    .respond(Response::from_string("Unable to create file").with_status_code(500)),
            }
        }
        Method::Delete => match remove_file(dest_path) {
            Ok(_) => req.respond(Response::from_string("Delete").with_status_code(204)),
            Err(_) => {
                req.respond(Response::from_string("Unable to delete file").with_status_code(500))
            }
        },
        _ => {
            req.respond(Response::from_string("not implemented").with_status_code(200))
        }
    };
}

pub fn start(port: u16, data_dir: String) {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let server = Arc::new(tiny_http::Server::http(addr).unwrap());
    let mut handles = Vec::new();

    // create data directory. files are initially created in tmp dir then moved to corresponding path
    if create_dir_all(Path::new(&data_dir).join("tmp")).is_err() {
        panic!("Could not create data dir. exiting\n");
    }

    let data_dir = Arc::new(data_dir);

    // TODO: defult to numcpus
    for _ in 0..4 {
        let server = server.clone();
        let data_dir = data_dir.clone();

        handles.push(thread::spawn(move || {
            for rq in server.incoming_requests() {
                req_handler(&data_dir, rq);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
