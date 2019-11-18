use rusty_loader::{load_file, parse_mcu, FileHint};

#[test]
fn ihex_same_as_elf() {
    let mcu = parse_mcu("TEENSYLC").unwrap();
    let (ihex_binary, ihex_len) =
        load_file("tests/blink.ihex", FileHint::IHEX, &mcu).expect("Failed to load Intel hex file");
    let (elf_binary, elf_len) =
        load_file("tests/blink", FileHint::ELF, &mcu).expect("Failed to load ELF file");

    assert_eq!(ihex_len, elf_len);
    assert_eq!(ihex_binary.len(), elf_binary.len());
    assert_eq!(ihex_binary, elf_binary);
}
