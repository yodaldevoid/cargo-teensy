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

pub fn supported_mcus() -> Vec<&'static str> {
    MCUS.iter()
        .map(|&(s, _)| s)
        .chain(ALIASES.iter().map(|&(s, _)| s))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_supported_mcus() {
        let expected_names = vec![
            "at90usb162",
            "atmega32u4",
            "at90usb646",
            "at90usb1286",
            "mkl26z64",
            "mk20dx128",
            "mk20dx256",
            "mk64fx512",
            "mk66fx1m0",
            "TEENSY2",
            "TEENSY2PP",
            "TEENSYLC",
            "TEENSY30",
            "TEENSY31",
            "TEENSY35",
            "TEENSY36",
        ];
        let names = supported_mcus();
        assert_eq!(expected_names, names);
    }
}
