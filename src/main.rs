use rusty_loader::{parse_mcu, supported_mcus};
use rusty_loader::usb::Teensy;

use clap::{App, Arg};

fn main() {
    let matches = App::new("rusty_loader")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"))
        .author("Gabriel \"yodaldevoid\" Smith <ga29smith@gmail.com>")
        .about("A rust rewrite of teensy_loader_cli")
        .arg(Arg::with_name("mcu")
            .long("mcu")
            .short("m")
            .help("The microcontroller to operate on")
            .takes_value(true)
            .empty_values(false)
            .required(true)
            .possible_values(&supported_mcus())
        )
        .arg(Arg::with_name("boot-only")
            .long("boot")
            .short("b")
            .help("Only boot the Teensy, do not program")
        )
        .get_matches();

    let mcu = match parse_mcu(matches.value_of("mcu").unwrap()) {
        Some(mcu) => mcu,
        None => {
            eprintln!("Unkown Teensy name");
            std::process::exit(1);
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

    if matches.is_present("boot-only") {
        // FIXME: Verbose
        println!("Booting");
        if let Err(err) = teensy.boot() {
            eprintln!("Boot failed");
            // FIXME: Verbose
            eprintln!("Boot error: {:?}", err);
            std::process::exit(1);
        }
    } else {
        unimplemented!()
    }
}
