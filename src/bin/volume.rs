use argparse::{ArgumentParser, Store};

fn main() {
    let mut port = 7000;
    let mut datadir = "/tmp/kalavarastore".to_string();
    let mut master = "localhost:6000".to_string();

    {
        // this block limits scope of borrows by ap.refer() method
        let mut cli = ArgumentParser::new();
        cli.set_description("kalavara volume server.");
        cli.refer(&mut port)
            .add_option(&["-p", "--port"], Store, "Port");
        cli.refer(&mut datadir)
            .add_option(&["-d", "--datadir"], Store, "Data directory");

        cli.refer(&mut master)
            .add_option(&["-m", "--master"], Store, "Master server to connect to");

        cli.parse_args_or_exit();
    }


    println!("port: {}, datadir: {}, master: {}", port, datadir, master);
}
