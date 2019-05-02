use md5::compute as compute_md5;
use tiny_http::{Method, Request, Response};

use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

fn req_handler(data_dir: &String, req: &mut Request) -> Response<Cursor<Vec<u8>>> {
    let path = format!("{:x}", compute_md5(req.url().as_bytes()));
    let mut body = String::new();
    let _ = req.as_reader().read_to_string(&mut body);

    let dest_dir = format!(
        "{}/{}/{}",
        data_dir,
        path.get(0..1).unwrap(),
        path.get(1..2).unwrap()
    );

    let filename = path.get(2..).unwrap();

    println!("{}/{}", dest_dir, filename);

    match req.method() {
           &Method::Get => unimplemented!(),
        _ => Response::from_string("volume server").with_status_code(200),
    }
}

pub fn start(port: u16, data_dir: String) {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let server = Arc::new(tiny_http::Server::http(addr).unwrap());
    let mut handles = Vec::new();
    let data_dir = Arc::new(data_dir);

    // TODO: defult to numcpus
    for _ in 0..4 {
        let server = server.clone();
        let data_dir = data_dir.clone();

        handles.push(thread::spawn(move || {
            for mut rq in server.incoming_requests() {
                let response = req_handler(&data_dir, &mut rq);
                let _ = rq.respond(response);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
