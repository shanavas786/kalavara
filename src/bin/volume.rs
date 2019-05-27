use argparse::{ArgumentParser, Store, StoreOption};

use kalavara::volume::start;
use std::process::exit;

fn main() {
    let mut port = 7000;
    let mut data_dir = "/tmp/kalavarastore".to_string();
    let mut threads = num_cpus::get() as u16;
    let mut master: Option<String> = None;
    let mut base: Option<String> = None;

    {
        // this block limits scope of borrows by ap.refer() method
        let mut cli = ArgumentParser::new();
        cli.set_description("kalavara volume server.");
        cli.refer(&mut port)
            .add_option(&["-p", "--port"], Store, "Port");
        cli.refer(&mut data_dir)
            .add_option(&["-d", "--data_dir"], Store, "Data directory");

        cli.refer(&mut threads).add_option(
            &["-t", "--threads"],
            Store,
            "Number of threads, defaults to number of cpu cores",
        );

        cli.refer(&mut master).add_option(
            &["-m", "--master"],
            StoreOption,
            "Master server to register at",
        );

        cli.refer(&mut base).add_option(
            &["-b", "--base-url"],
            StoreOption,
            "Base url of server to register with master",
        );

        cli.parse_args_or_exit();
    }

    if master.is_some() && base.is_none() {
        eprintln!("base url is required to register with master");
        exit(2);
    }

    // remove trailing slash
    if data_dir.ends_with('/') {
        data_dir.pop();
    }

    println!(
        "port: {}, data_dir: {}, threads: {}, master: {:?}",
        port, data_dir, threads, master
    );

    start(port, data_dir, threads, master, base);
}
