use tempfile::tempdir;

use kalavara::master::start as master_start;
use kalavara::volume::start as volume_start;

use std::sync::{Once, ONCE_INIT};
use std::thread;
use std::time::Duration;

static INIT: Once = ONCE_INIT;

fn run() {
    let master_data_dir = tempdir().unwrap();
    let volume_data_dir = tempdir().unwrap();

    thread::spawn(move || {
        master_start(
            6000,
            master_data_dir.path().to_str().unwrap(),
            4,
            vec!["http://localhost:7000".to_string()],
        );
    });

    thread::spawn(move || {
        volume_start(
            7000,
            volume_data_dir.path().to_str().unwrap().to_owned(),
            4,
            None,
            None,
        );
    });
}

/// Setup function that is only run once, even if called multiple times.
/// to be called by all tests.
fn setup() {
    INIT.call_once(|| {
        run();
        thread::sleep(Duration::from_millis(1000));
    });
}

#[test]
fn test_kv() {
    setup();

    let res = minreq::put("http://localhost:6000/store/key1")
        .with_body("val1")
        .send();

    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 201);

    let res = minreq::get("http://localhost:6000/store/key1").send();
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.status_code, 200);
    assert_eq!(res.body, "val1");

    let res = minreq::delete("http://localhost:6000/store/key1").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 204);

    let res = minreq::get("http://localhost:6000/store/key1").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 404);
}

#[test]
fn test_remove_query_params() {
    setup();
    let res = minreq::put("http://localhost:6000/store/key2?query=value")
        .with_body("val2")
        .send();

    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 201);

    let res = minreq::get("http://localhost:6000/store/key2?que=valu").send();
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.status_code, 200);
    assert_eq!(res.body, "val2");

    let res = minreq::delete("http://localhost:6000/store/key2?q=v").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 204);

    let res = minreq::get("http://localhost:6000/store/key2").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 404);
}

#[test]
fn test_no_prefix() {
    setup();
    // without store prefix
    let res = minreq::put("http://localhost:6000/key2?query=value")
        .with_body("val2")
        .send();

    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 404);
}
