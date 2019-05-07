use argparse::{ArgumentParser, List, Store};
use kalavara::master;

fn main() {
    let mut port: u16 = 6000;
    let mut data_dir = "/tmp/kalavaradb".to_string();
    let mut volumes: Vec<String> = Vec::new();
    let mut threads = num_cpus::get() as u16;

    {
        // this block limits scope of borrows by ap.refer() method
        let mut cli = ArgumentParser::new();
        cli.set_description("kalavara master server.");
        cli.refer(&mut port)
            .add_option(&["-p", "--port"], Store, "Port");

        cli.refer(&mut data_dir)
            .add_option(&["-d", "--data_dir"], Store, "Database directory");

        cli.refer(&mut threads).add_option(
            &["-t", "--threads"],
            Store,
            "Number of threads, defaults to number of cpu cores",
        );

        cli.refer(&mut volumes)
            .add_option(&["-v", "--volumes"], List, "Volumes");

        cli.parse_args_or_exit();
    }

    // remote trailing slashes from volume server urls
    for volume in volumes.iter_mut() {
        if volume.ends_with('/') {
            volume.pop();
        }
    }

    println!(
        "port: {}, data_dir: {}, threads: {}, volumes: {:?}",
        port, data_dir, threads, volumes
    );

    master::start(port, &data_dir, threads, volumes);
}
