use kalavara::master::start as master_start;
use kalavara::volume::start as volume_start;

use std::sync::{Once, ONCE_INIT};
use std::thread;
use std::time::Duration;

mod basic;

static INIT: Once = ONCE_INIT;

fn run() {
    thread::spawn(move || {
        master_start(6001, "/tmp/master1/", 4, vec![]);
    });

    thread::spawn(move || {
        volume_start(7001, "/tmp/volume1/".to_string(), 4, None, None);
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
fn test_add_volume() {
    setup();

    // no volume servers registered
    let res = minreq::put("http://localhost:6001/store/key1")
        .with_body("val1")
        .send();

    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 503);

    // add volume
    let res = minreq::post("http://localhost:6001/admin/add-volume")
        .with_body("http://localhost:7001")
        .send();

    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.body, "Volume added");

    // trying to insert again
    let res = minreq::post("http://localhost:6001/admin/add-volume")
        .with_body("http://localhost:7001")
        .send();

    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.body.trim(), "Skipping duplicate volume server");

    // api tests
    let res = minreq::put("http://localhost:6001/store/key1")
        .with_body("val1")
        .send();

    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 201);

    let res = minreq::get("http://localhost:6001/store/key1").send();
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.status_code, 200);
    assert_eq!(res.body.trim(), "val1");

    let res = minreq::delete("http://localhost:6001/store/key1").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 204);

    let res = minreq::get("http://localhost:6001/store/key1").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 404);
}
