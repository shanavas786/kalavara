use argparse::{ArgumentParser, Store};

use kalavara::volume::start;

fn main() {
    let mut port = 7000;
    let mut data_dir = "/tmp/kalavarastore".to_string();
    let mut threads = num_cpus::get() as u16;
    // let mut master = "".to_string();

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

        // cli.refer(&mut master).add_option(
        //     &["-m", "--master"],
        //     Store,
        //     "Master server to connect to",
        // );

        cli.parse_args_or_exit();
    }

    // remove trailing slash
    if data_dir.ends_with('/') {
        data_dir.pop();
    }

    println!(
        "port: {}, data_dir: {}, threads: {}",
        port, data_dir, threads
    );

    start(port, data_dir, threads);
}
