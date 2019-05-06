use std::fs::File;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;

use rusty_loader::{ihex_to_bytes, parse_mcu, supported_mcus};
use rusty_loader::usb::{ConnectError, Teensy};

use clap::{App, Arg};
use ihex::reader::Reader as IHexReader;
use ihex::record::Record as IHexRecord;

static mut VERBOSE: bool = false;

macro_rules! println_verbose {
    ($($arg:tt)*) => ({
        if unsafe { VERBOSE } {
            println!($($arg)*);
        }
    })
}

macro_rules! print_verbose {
    ($($arg:tt)*) => ({
        if unsafe { VERBOSE } {
            print!($($arg)*);
        }
    })
}

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
        .arg(Arg::with_name("verbose")
            .long("verbose")
            .short("v")
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
        .arg(Arg::with_name("file")
            .required_unless("boot-only")
        )
        .get_matches();

    let mcu = match parse_mcu(matches.value_of("mcu").unwrap()) {
        Some(mcu) => mcu,
        None => {
            eprintln!("Unkown device name");
            std::process::exit(1);
        }
    };

    unsafe {
        VERBOSE = matches.is_present("verbose");
    }

    let boot_only = matches.is_present("boot-only");

    let binary = if !boot_only {
        let file_path = matches.value_of("file").expect("No file path though boot-only not set");
        match File::open(file_path) {
            Ok(mut file) => {
                // Check the binary size
                let mut file_str = String::new();
                if let Err(err) = file.read_to_string(&mut file_str) {
                    eprintln!("Failed to read \"{:?}\"", file_path);
                    println_verbose!("Error: {}", err);
                    std::process::exit(1);
                }
                let ihex_reader = IHexReader::new(&file_str);
                let ihex_records: Result<Vec<_>, _> = ihex_reader.collect();
                let ihex_records = match ihex_records {
                    Ok(r) => r,
                    Err(err) => {
                        eprintln!("Failed to parse \"{}\" as Intel hex", file_path);
                        println_verbose!("Error: {}", err);
                        std::process::exit(1);
                    }
                };
                let len: usize = ihex_records.iter()
                    .map(|rec| if let IHexRecord::Data { value, .. } = rec {
                        value.len()
                    } else {
                        0
                    })
                    .sum();

                println_verbose!(
                    "Read \"{}\": {} bytes, {:.*}% usage",
                    file_path,
                    len,
                    1,
                    len as f64 / mcu.0 as f64 * 100.0
                );

                match ihex_to_bytes(&ihex_records, mcu.0) {
                    Ok(binary) => Some(binary),
                    Err(_) => {
                        eprintln!("Failed to parse \"{}\" into binary form", file_path);
                        std::process::exit(1);
                    }
                }
            }
            Err(err) => {
                eprintln!("Failed to open \"{}\"", file_path);
                println_verbose!("Error: {}", err);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let wait_for_device = matches.is_present("wait");
    let mut waited = false;
    let mut teensy = loop {
        match Teensy::connect(mcu.0, mcu.1) {
            Ok(t) => break t,
            Err(err) => {
                if err == ConnectError::DeviceNotFound && !wait_for_device {
                    eprintln!("Unable to open device (hint: try --wait)");
                    std::process::exit(1);
                } else if err != ConnectError::DeviceNotFound {
                    println_verbose!("Connection error: {:?}", err);
                    std::process::exit(1);
                }
            }
        }
        if !waited {
            println_verbose!("Waiting for device...");
            println_verbose!(" (hint: press the reset button)");
            waited = true;
        }
        sleep(Duration::from_millis(250));
    };

    println_verbose!("Found HalfKey Bootloader");

    if !boot_only {
        if let Some(binary) = binary {
            println_verbose!("Programming");

            let binary_chunks = binary.chunks_exact(mcu.1);
            if !binary_chunks.remainder().is_empty() {
                panic!("Somehow the addressed binary had a remainder")
            }

            let mut buf = if mcu.1 == 256 {
                Vec::with_capacity(mcu.1 + 2)
            } else if mcu.1 == 512 || mcu.1 == 1024 {
                Vec::with_capacity(mcu.1 + 64)
            } else {
                eprintln!("Unknown code/black size");
                println_verbose!("code/block: {}/{}", mcu.0, mcu.1);
                std::process::exit(1);
            };

            for (addr, chunk) in (0..mcu.0).step_by(mcu.1).zip(binary_chunks) {
                if addr != 0 && chunk.iter().all(|&x| x == 0xFF) {
                    continue;
                }

                print_verbose!(".");

                if mcu.1 <= 256 {
                    buf.resize(2, 0);
                    if mcu.0 < 0x10000 {
                        buf[0] = addr as u8;
                        buf[1] = (addr >> 8) as u8;
                    } else {
                        buf[0] = (addr >> 8) as u8;
                        buf[1] = (addr >> 16) as u8;
                    }
                    buf.extend_from_slice(chunk);
                } else if mcu.1 == 512 || mcu.1 == 1024 {
                    buf.resize(64, 0);
                    buf[0] = addr as u8;
                    buf[1] = (addr >> 8) as u8;
                    buf[2] = (addr >> 16) as u8;
                    buf.extend_from_slice(chunk);
                } else {
                    eprintln!("Unknown code/black size");
                    println_verbose!("code/block: {}/{}", mcu.0, mcu.1);
                    std::process::exit(1);
                };

                if let Err(err) = teensy.write(
                    &buf,
                    Duration::from_millis(if addr == 0 { 5000 } else { 500 })
                ) {
                    eprintln!("Error writing to Teensy");
                    println_verbose!("Error: {:?}", err);
                    std::process::exit(1);
                }
            }
        }
    }

    println_verbose!("Booting");
    if let Err(err) = teensy.boot() {
        eprintln!("Boot failed");
        println_verbose!("Boot error: {:?}", err);
        std::process::exit(1);
    }
}
