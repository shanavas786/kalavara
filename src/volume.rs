//! # volume server
//!
//! Volume server stores values in file system. For atomicity temporary files are
//! first created in `destdir/tmp` directory and then moved to destination path.
//! For this approach to work, `destdir/tmp` and destination path should be in same
//! file system
//!
//! to start the volume server, run
//!
//! ```sh
//! volume -p 7000 -d /tmp/kalavarastore
//! ```

use md5::compute as compute_md5;
use tempfile::NamedTempFile;
use tiny_http::{Request, Response};

use std::fs::{create_dir_all, remove_file, File};
use std::io::{copy, Error, ErrorKind, Read};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use crate::{Respond, Service};

/// volume store
struct Volume {
    /// directory to store file blobs
    data_dir: Arc<String>,
}

/// Types of responses that master generates
enum ResponseKind {
    /// Path to file blob
    FilePath(PathBuf),

    /// Value saved
    Created,

    /// Value deleted
    Deleted,

    /// Error occured, 500
    ServerError,

    /// Method not allowed, 405
    NotAllowed,
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
            FilePath(path) => match File::open(path) {
                Ok(file) => req.respond(Response::from_file(file)),
                Err(_) => req.respond(resp!("Server Error", 500)),
            },
            Created => req.respond(resp!("Created", 201)),
            Deleted => req.respond(resp!("Deleted", 204)),
            ServerError => req.respond(resp!("Server error", 500)),
            NotAllowed => req.respond(resp!("Method not allowd", 405)),
        };
    }
}

impl Volume {
    /// Create new volume service
    fn new(data_dir: String) -> Self {
        Self {
            data_dir: Arc::new(data_dir),
        }
    }

    /// Calcualtes destination file path from key
    fn key_to_path(&self, key: &str) -> PathBuf {
        let path = format!("{:x}", compute_md5(key.as_bytes()));

        let mut dest_path = PathBuf::from(self.data_dir.as_ref());
        dest_path.push(path.get(0..1).unwrap());
        dest_path.push(path.get(1..2).unwrap());
        dest_path.push(path.get(2..).unwrap());

        dest_path
    }
}

impl Service for Volume {
    type Response = ResponseKind;

    /// Get value of a key from store
    fn get(&self, key: String) -> Self::Response {
        let dest_path = self.key_to_path(&key);
        ResponseKind::FilePath(dest_path)
    }

    /// Save/Update key in store
    fn save(&self, key: String, mut value: impl Read) -> Self::Response {
        let tmpdir = Path::new(self.data_dir.as_ref()).join("tmp");
        let dest_path = self.key_to_path(&key);

        match NamedTempFile::new_in(tmpdir) {
            Ok(mut tmpfile) => {
                // copy data to file
                match copy(&mut value, &mut tmpfile)
                    .and(create_dir_all(dest_path.parent().unwrap()))
                    .and(
                        tmpfile
                            .persist(dest_path)
                            .map_err(|_| Error::new(ErrorKind::Other, "")),
                    ) {
                    Ok(_) => ResponseKind::Created,
                    _ => ResponseKind::ServerError,
                }
            }
            Err(_) => ResponseKind::ServerError,
        }
    }

    /// Remove a key from store
    fn delete(&self, key: String) -> Self::Response {
        let dest_path = self.key_to_path(&key);

        match remove_file(dest_path) {
            Ok(_) => ResponseKind::Deleted,
            Err(_) => ResponseKind::ServerError,
        }
    }
}

pub fn start(port: u16, data_dir: String, threads: u16) {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let server = Arc::new(tiny_http::Server::http(addr).unwrap());
    let mut handles = Vec::new();

    // creates data directory. files are initially created in tmp dir then moved to corresponding
    // path
    if create_dir_all(Path::new(&data_dir).join("tmp")).is_err() {
        panic!("Could not create data dir. exiting\n");
    }

    let volume = Arc::new(Volume::new(data_dir));

    for _ in 0..threads {
        let server = server.clone();
        let handler = volume.clone();

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
