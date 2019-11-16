use ihex::record::Record as IHexRecord;

pub mod usb;

#[derive(Clone, Copy, Debug)]
pub struct Mcu {
    pub code_size: usize,
    pub block_size: usize,
}

/// MCU name, flash size, block size
static MCUS: [(&'static str, Mcu); 9] = [
    (
        "at90usb162",
        Mcu {
            code_size: 15872,
            block_size: 128,
        },
    ),
    (
        "atmega32u4",
        Mcu {
            code_size: 32256,
            block_size: 128,
        },
    ),
    (
        "at90usb646",
        Mcu {
            code_size: 64512,
            block_size: 256,
        },
    ),
    (
        "at90usb1286",
        Mcu {
            code_size: 130048,
            block_size: 256,
        },
    ),
    (
        "mkl26z64",
        Mcu {
            code_size: 63488,
            block_size: 512,
        },
    ),
    (
        "mk20dx128",
        Mcu {
            code_size: 131072,
            block_size: 1024,
        },
    ),
    (
        "mk20dx256",
        Mcu {
            code_size: 262144,
            block_size: 1024,
        },
    ),
    (
        "mk64fx512",
        Mcu {
            code_size: 524288,
            block_size: 1024,
        },
    ),
    (
        "mk66fx1m0",
        Mcu {
            code_size: 1048576,
            block_size: 1024,
        },
    ),
];

/// Alias name, MCU name
static ALIASES: [(&'static str, &'static str); 8] = [
    ("TEENSY2", "atmega32u4"),
    ("TEENSY2PP", "at90usb1286"),
    ("TEENSYLC", "mkl26z64"),
    ("TEENSY30", "mk20dx128"),
    ("TEENSY31", "mk20dx256"),
    ("TEENSY32", "mk20dx256"),
    ("TEENSY35", "mk64fx512"),
    ("TEENSY36", "mk66fx1m0"),
];

// FIXME:
pub fn parse_mcu(arg: &str) -> Option<Mcu> {
    let name = ALIASES
        .iter()
        .filter(|&&(alias, _)| alias == arg)
        .next()
        .map(|&(_, n)| n)
        .unwrap_or(arg);

    MCUS.iter()
        .filter(|(n, ..)| *n == name)
        .next()
        .map(|&(_, mcu)| mcu)
}

pub fn supported_mcus() -> Vec<&'static str> {
    MCUS.iter()
        .map(|&(s, ..)| s)
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
            "TEENSY32",
            "TEENSY35",
            "TEENSY36",
        ];
        let names = supported_mcus();
        assert_eq!(expected_names, names);
    }
}

pub fn ihex_to_bytes(recs: &[IHexRecord], mcu: &Mcu) -> Result<Vec<u8>, ()> {
    let mut base_address = 0;
    let mut bytes = vec![0xFF; mcu.code_size];

    for rec in recs {
        match rec {
            IHexRecord::Data { offset, value } => {
                if base_address + *offset as usize + value.len() >= mcu.code_size {
                    return Err(());
                }

                for (n, b) in value.iter().enumerate() {
                    bytes[base_address + *offset as usize + n] = *b;
                }
            }
            IHexRecord::ExtendedSegmentAddress(base) => base_address = (*base as usize) << 4,
            IHexRecord::ExtendedLinearAddress(base) => base_address = (*base as usize) << 16,
            IHexRecord::EndOfFile => break,
            IHexRecord::StartLinearAddress(_) | IHexRecord::StartSegmentAddress { .. } => {
                return Err(())
            }
        }
    }

    Ok(bytes)
}
