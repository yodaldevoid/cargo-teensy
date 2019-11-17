use std::fs::File;
use std::io::{Error as IoError, Read};

use elf_rs::{
    Elf, Elf32, ElfAbi, ElfMachine, ElfType, GenElf, GenElfHeader, GenProgramHeader,
    GenSectionHeader, ProgramHeader32, ProgramType, SectionHeader, SectionHeaderFlags, SectionType,
};
use ihex::reader::Reader as IHexReader;
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileHint {
    IHEX,
    ELF,
    Any,
}

impl FileHint {
    pub fn to_str(&self) -> &'static str {
        match self {
            FileHint::IHEX => "Intel hex",
            FileHint::ELF => "ELF",
            FileHint::Any => "Intel hex or ELF",
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    FailedOpen(IoError),
    FailedRead(IoError),
    NotValidFile,
}

pub fn load_file(
    file_path: &str,
    hint: FileHint,
    mcu: &Mcu,
) -> Result<(Vec<u8>, usize), LoadError> {
    let mut file = File::open(file_path).map_err(|e| LoadError::FailedOpen(e))?;
    let mut file_buf = Vec::new();
    file.read_to_end(&mut file_buf)
        .map_err(|e| LoadError::FailedRead(e))?;

    // Assume the file is an ELF file first. If that fails to parse, try IHEX.
    if hint != FileHint::IHEX {
        match Elf::from_bytes(&file_buf[..]) {
            // TODO: Return errors
            Ok(Elf::Elf32(elf)) => {
                if elf.header().machine() != ElfMachine::ARM {
                    None
                } else if elf.header().abi() != ElfAbi::SystemV {
                    // SystemV is used as None
                    None
                } else if elf.header().elftype() != ElfType::ET_EXEC {
                    None
                } else if elf.program_headers().iter().any(|phdr| {
                    phdr.ph_type() == ProgramType::DYNAMIC || phdr.ph_type() == ProgramType::INTERP
                }) {
                    None
                } else {
                    elf32_to_bytes(&elf, mcu).ok()
                    //eprintln!("Failed to parse \"{}\" into binary form", file_path);
                    //println_verbose!("Error: {:?}", err);
                }
            }
            _ => None,
        }
    } else {
        None
    }
    .or_else(|| {
        if hint != FileHint::ELF {
            let file_str = String::from_utf8_lossy(&file_buf[..]);
            let ihex_reader = IHexReader::new(&file_str);
            let ihex_records: Result<Vec<_>, _> = ihex_reader.collect();
            match ihex_records {
                Ok(r) => Some(r),
                Err(_err) => {
                    //eprintln!("Failed to parse \"{}\" as Intel hex", file_path);
                    //println_verbose!("Error: {}", err);
                    None
                }
            }
            .and_then(|ihex_records| {
                match ihex_to_bytes(&ihex_records, mcu) {
                    Err(_err) => {
                        //eprintln!("Failed to parse \"{}\" into binary form", file_path);
                        //println_verbose!("Error: {:?}", err);
                        None
                    }
                    Ok(bin) => Some(bin),
                }
            })
        } else {
            None
        }
    })
    .ok_or(LoadError::NotValidFile)
}

#[derive(Debug, PartialEq)]
pub enum IHexError {
    AddressTooHigh(usize),
}

pub fn ihex_to_bytes(recs: &[IHexRecord], mcu: &Mcu) -> Result<(Vec<u8>, usize), IHexError> {
    let mut base_address = 0;
    let mut bytes = vec![0xFF; mcu.code_size];
    let mut len = 0;

    for rec in recs {
        match rec {
            IHexRecord::Data { offset, value } => {
                let end_addr = base_address + *offset as usize + value.len();
                if end_addr >= mcu.code_size {
                    return Err(IHexError::AddressTooHigh(end_addr));
                }

                len += value.len();
                for (n, b) in value.iter().enumerate() {
                    bytes[base_address + *offset as usize + n] = *b;
                }
            }
            IHexRecord::ExtendedSegmentAddress(base) => base_address = (*base as usize) << 4,
            IHexRecord::ExtendedLinearAddress(base) => base_address = (*base as usize) << 16,
            IHexRecord::EndOfFile => break,
            // Defines the start location for our program. This doesn't concern us so we ignore it.
            IHexRecord::StartLinearAddress(_) | IHexRecord::StartSegmentAddress { .. } => {}
        }
    }

    Ok((bytes, len))
}

struct Section<'a> {
    shdr: SectionHeader<'a, Elf32<'a>>,
    load_addr: u32,
    size: u32,
}

impl<'a, 'b> Section<'a> {
    fn new(sec: SectionHeader<'a, Elf32<'a>>, phdrs: &'b [ProgramHeader32]) -> Self {
        let shdr = sec.sh;

        if let Some(phdr) = phdrs.iter().find(|phdr| {
            shdr.addr() >= phdr.vaddr() && shdr.addr() + shdr.size() <= phdr.vaddr() + phdr.memsz()
        }) {
            Section {
                shdr: sec,
                load_addr: shdr.addr() - phdr.vaddr() + phdr.paddr(),
                size: shdr.size(),
            }
        } else {
            Section {
                shdr: sec,
                load_addr: shdr.addr(),
                size: shdr.size(),
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ElfError {}

// TODO: verify nothing is above the MCU's code size
pub fn elf32_to_bytes(elf: &Elf32, _mcu: &Mcu) -> Result<(Vec<u8>, usize), ElfError> {
    let sections: Vec<_> = elf
        .section_header_iter()
        .filter(|s| {
            s.sh.sh_type() == SectionType::SHT_PROGBITS
                && s.sh.flags().contains(SectionHeaderFlags::SHF_ALLOC)
        })
        .map(|s| Section::new(s, elf.program_headers()))
        .collect();

    let base_addr = sections.iter().map(|s| s.load_addr as usize).min().unwrap();
    let end_addr = sections
        .iter()
        .map(|s| (s.load_addr + s.size) as usize)
        .max()
        .unwrap();
    let size = end_addr - base_addr;

    let mut data = vec![0; size];
    let mut len = 0;

    for section in sections {
        let start = section.load_addr as usize - base_addr;
        let end = start + section.size as usize;
        len += end - start;
        data[start..end].copy_from_slice(section.shdr.segment());
    }
    Ok((data, len))
}
