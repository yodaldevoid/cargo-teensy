use std::fs::File;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;

use clap::{App, Arg};
use elf_rs::{
    Elf, ElfAbi, ElfMachine, ElfType, GenElf, GenElfHeader, GenProgramHeader, ProgramType,
};
use ihex::reader::Reader as IHexReader;

use rusty_loader::usb::{ConnectError, ProgramError, Teensy};
use rusty_loader::{elf32_to_bytes, ihex_to_bytes, parse_mcu, supported_mcus};

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
        .arg(
            Arg::with_name("mcu")
                .long("mcu")
                .short("m")
                .help("The microcontroller to operate on")
                .takes_value(true)
                .empty_values(false)
                .required(true)
                .possible_values(&supported_mcus()),
        )
        .arg(Arg::with_name("verbose").long("verbose").short("v"))
        .arg(
            Arg::with_name("wait")
                .long("wait")
                .short("w")
                .help("Wait for the device to appear"),
        )
        .arg(
            Arg::with_name("no-reboot")
                .long("no-reboot")
                .short("n")
                .help("No reboot after programming")
                .requires("file"),
        )
        .arg(
            Arg::with_name("boot-only")
                .long("boot")
                .short("b")
                .help("Only boot the device, do not program"),
        )
        .arg(
            Arg::with_name("elf")
                .long("elf")
                .short("e")
                .help("Input file should be treated as an ELF file")
                .conflicts_with("ihex")
                .conflicts_with("boot-only"),
        )
        .arg(
            Arg::with_name("ihex")
                .long("ihex")
                .short("i")
                .help("Input file should be treated as an Intel HEX file")
                .conflicts_with("elf")
                .conflicts_with("boot-only"),
        )
        .arg(
            Arg::with_name("file")
                .conflicts_with("boot-only")
                .required_unless("boot-only"),
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
        let file_path = matches
            .value_of("file")
            .expect("No file path though boot-only not set");
        match File::open(file_path) {
            Ok(mut file) => {
                let mut file_buf = Vec::new();
                if let Err(err) = file.read_to_end(&mut file_buf) {
                    eprintln!("Failed to read \"{:?}\"", file_path);
                    println_verbose!("Error: {}", err);
                    std::process::exit(1);
                }

                // Assume the file is an ELF file first. If that fails to parse, try IHEX.
                if let Some((binary, len)) = if !matches.is_present("ihex") {
                    match Elf::from_bytes(&file_buf[..]) {
                        // TODO: Print error
                        Ok(Elf::Elf32(elf)) => {
                            if elf.header().machine() != ElfMachine::ARM {
                                None
                            } else if elf.header().abi() != ElfAbi::SystemV {
                                // SystemV is used as None
                                None
                            } else if elf.header().elftype() != ElfType::ET_EXEC {
                                None
                            } else if elf.program_headers().iter().any(|phdr| {
                                phdr.ph_type() == ProgramType::DYNAMIC
                                    || phdr.ph_type() == ProgramType::INTERP
                            }) {
                                None
                            } else {
                                elf32_to_bytes(&elf, &mcu).ok().or_else(|| {
                                    eprintln!(
                                        "Failed to parse \"{}\" into binary form",
                                         file_path,
                                    );
                                    std::process::exit(1);
                                })
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                }
                .or_else(|| {
                    if !matches.is_present("elf") {
                        let file_str = String::from_utf8_lossy(&file_buf[..]);
                        let ihex_reader = IHexReader::new(&file_str);
                        let ihex_records: Result<Vec<_>, _> = ihex_reader.collect();
                        match ihex_records {
                            Ok(r) => Some(r),
                            Err(err) => {
                                eprintln!("Failed to parse \"{}\" as Intel hex", file_path);
                                println_verbose!("Error: {}", err);
                                None
                            }
                        }
                        .and_then(|ihex_records| {
                            match ihex_to_bytes(&ihex_records, &mcu) {
                                Err(err) => {
                                    eprintln!("Failed to parse \"{}\" into binary form", file_path);
                                    println_verbose!("Error: {:?}", err);
                                    None
                                }
                                Ok(bin) => Some(bin),
                            }
                        })
                    } else {
                        None
                    }
                }) {
                    println_verbose!(
                        "Read \"{}\": {} bytes, {:.*}% usage",
                        file_path,
                        len,
                        1,
                        len as f64 / mcu.code_size as f64 * 100.0
                    );

                    Some(binary)
                } else {
                    let file_types = match (matches.is_present("ihex"), matches.is_present("elf")) {
                        (true, false) => "Intel hex",
                        (false, true) => "ELF",
                        _ => "Intel hex or ELF",
                    };
                    eprintln!(
                        "\"{}\" does not seem to be an {} file",
                        file_path, file_types
                    );
                    std::process::exit(1);
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
        match Teensy::connect(mcu) {
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

            if let Err(err) = teensy.program(&binary, |_| print_verbose!(".")) {
                match err {
                    ProgramError::BinaryRemainder => {
                        panic!("Somehow the addressed binary had a remainder")
                    }
                    ProgramError::UnknownBlockSize(size) => {
                        eprintln!("Unknown block size");
                        println_verbose!("block: {}", size);
                        std::process::exit(1);
                    }
                    ProgramError::WriteError(err) => {
                        eprintln!("Error writing to Teensy");
                        println_verbose!("Error: {:?}", err);
                        std::process::exit(1);
                    }
                }
            }

            println_verbose!();
        }
    }

    if !matches.is_present("no-reboot") || boot_only {
        println_verbose!("Booting");
        if let Err(err) = teensy.boot() {
            eprintln!("Boot failed");
            println_verbose!("Boot error: {:?}", err);
            std::process::exit(1);
        }
    }
}
