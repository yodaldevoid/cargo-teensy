use rusty_loader::supported_mcus;

use clap::{App, Arg};

fn main() {
    let matches = App::new("rusty_loader")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"))
        .author("Gabriel \"yodaldevoid\" Smith <ga29smith@gmail.com>")
        .about("A rust rewrite of teensy_loader_cli")
        .arg(Arg::with_name("list-mcus")
            .long("list-mcus")
            .help("Lists supported MCUs")
        )
        .get_matches();

    if matches.is_present("list-mcus") {
        print_supported_mcus();
        return;
    }
}

fn print_supported_mcus() {
    println!("Supported MCUs are:");
    for name in supported_mcus() {
        println!(" - {}", name);
    }
}
