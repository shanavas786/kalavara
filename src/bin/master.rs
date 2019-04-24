use argparse::{ArgumentParser, Store};
use kalavara::master;

fn main() {
    let mut port: u16 = 6000;
    let mut data_dir = "/tmp/kalavaradb".to_string();

    {
        // this block limits scope of borrows by ap.refer() method
        let mut cli = ArgumentParser::new();
        cli.set_description("kalavara master server.");
        cli.refer(&mut port)
            .add_option(&["-p", "--port"], Store, "Port");
        cli.refer(&mut data_dir)
            .add_option(&["-d", "--data_dir"], Store, "Database directory");
        cli.parse_args_or_exit();
    }

    println!("port: {}, data_dir: {}", port, data_dir);

    master::start(port, &data_dir);
}
