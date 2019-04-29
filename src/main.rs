fn main() {
    print_supported_mcus();
}

/// MCU name, flash size
static MCUS: [(&'static str, usize); 9] = [
    ("at90usb162", 15872),
    ("atmega32u4", 32256),
    ("at90usb646", 64512),
    ("at90usb1286", 130048),
    ("mkl26z64", 63488),
    ("mk20dx128", 131072),
    ("mk20dx256", 262144),
    ("mk64fx512", 524288),
    ("mk66fx1m0", 1048576),
];

/// Alias name, MCU name
static ALIASES: [(&'static str, &'static str); 7] = [
    ("TEENSY2", "atmega32u4"),
    ("TEENSY2PP", "at90usb1286"),
    ("TEENSYLC", "mkl26z64"),
    ("TEENSY30", "mk20dx128"),
    ("TEENSY31", "mk20dx256"),
    ("TEENSY35", "mk64fx512"),
    ("TEENSY36", "mk66fx1m0"),
];

fn print_supported_mcus() {
    println!("Supported MCUs are:");
    MCUS.iter()
        .map(|&(s, _)| s)
        .chain(ALIASES.iter().map(|&(s, _)| s))
        .for_each(|s| println!(" - {}", s));
}
