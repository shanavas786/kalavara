use kalavara::master::start as master_start;
use kalavara::volume::start as volume_start;

use std::sync::{Once, ONCE_INIT};
use std::thread;
use std::time::Duration;

static INIT: Once = ONCE_INIT;

fn run() {
    thread::spawn(move || {
        master_start(6002, "/tmp/master2/", 4, vec![]);
    });

    thread::sleep(Duration::from_millis(1000));

    thread::spawn(move || {
        volume_start(
            7002,
            "/tmp/volume2/".to_string(),
            4,
            Some("http://localhost:6002".to_string()),
            Some("http://localhost:7002".to_string()),
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
fn test_volume_auto_reg() {
    setup();

    // api tests
    let res = minreq::put("http://localhost:6002/store/key1")
        .with_body("val1")
        .send();

    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 201);

    let res = minreq::get("http://localhost:6002/store/key1").send();
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.status_code, 200);
    assert_eq!(res.body.trim(), "val1");

    let res = minreq::delete("http://localhost:6002/store/key1").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 204);

    let res = minreq::get("http://localhost:6002/store/key1").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status_code, 404);
}
