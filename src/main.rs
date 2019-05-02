use std::thread::sleep;
use std::time::Duration;

use rusty_loader::{parse_mcu, supported_mcus};
use rusty_loader::usb::{ConnectError, Teensy};

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
        .arg(Arg::with_name("wait")
            .long("wait")
            .short("w")
            .help("Wait for the device to appear")
        )
        .arg(Arg::with_name("boot-only")
            .long("boot")
            .short("b")
            .help("Only boot the device, do not program")
        )
        .get_matches();

    let mcu = match parse_mcu(matches.value_of("mcu").unwrap()) {
        Some(mcu) => mcu,
        None => {
            eprintln!("Unkown device name");
            std::process::exit(1);
        }
    };

    let wait_for_device = matches.is_present("wait");
    let mut waited = false;
    let teensy = loop {
        match Teensy::connect(mcu.0, mcu.1) {
            Ok(t) => break t,
            Err(err) => {
                if err == ConnectError::DeviceNotFound && !wait_for_device {
                    eprintln!("Unable to open device (hint: try --wait)");
                    std::process::exit(1);
                } else if err != ConnectError::DeviceNotFound {
                    // FIXME: Verbose
                    eprintln!("Connection error: {:?}", err);
                    std::process::exit(1);
                }
            }
        }
        if !waited {
            // FIXME: Verbose
            eprintln!("Waiting for device...");
            eprintln!(" (hint: press the reset button)");
            waited = true;
        }
        sleep(Duration::from_millis(250));
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
