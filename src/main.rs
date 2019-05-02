use rusty_loader::{parse_mcu, supported_mcus};
use rusty_loader::usb::Teensy;

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

    let mcu = match parse_mcu("") {
        Some(mcu) => mcu,
        None => {
            unimplemented!()
        }
    };

    let teensy = loop {
        match Teensy::connect(mcu.0, mcu.1) {
            Ok(t) => break t,
            Err(err) => {
                eprintln!("Unable to open device");
                // FIXME: Verbose
                eprintln!("Connection error: {:?}", err);
                std::process::exit(1);
            }
        }
    };

    println!("Found HalfKey Bootloader");

    // boot only
    // FIXME: Verbose
    println!("Booting");
    if let Err(err) = teensy.boot() {
        eprintln!("Boot failed");
        // FIXME: Verbose
        eprintln!("Boot error: {:?}", err);
        std::process::exit(1);
    }
}

fn print_supported_mcus() {
    println!("Supported MCUs are:");
    for name in supported_mcus() {
        println!(" - {}", name);
    }
}
