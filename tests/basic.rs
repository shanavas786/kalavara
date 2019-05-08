use kalavara::master::start as master_start;
use kalavara::volume::start as volume_start;
use reqwest::Client;

use std::thread;
use std::time::Duration;

fn run() {
    thread::spawn(move || {
        master_start(
            6000,
            "/tmp/master/",
            4,
            vec!["http://localhost:7000".to_string()],
        );
    });

    thread::spawn(move || {
        volume_start(7000, "/tmp/volume/".to_string(), 4);
    });
}

#[test]
fn test_kv() {
    run();

    // let ther servers ready
    thread::sleep(Duration::from_millis(1000));

    let res = Client::new()
        .put("http://localhost:6000/store/key1")
        .body("val1")
        .send();

    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), 201);

    let res = Client::new().get("http://localhost:6000/store/key1").send();
    assert!(res.is_ok());
    let mut res = res.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().unwrap(), "val1");

    let res = Client::new()
        .delete("http://localhost:6000/store/key1")
        .send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), 204);

    let res = Client::new().get("http://localhost:6000/store/key1").send();
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), 404);
}
