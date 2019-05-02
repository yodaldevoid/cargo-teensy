use std::time::Duration;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::*;

#[cfg(all(unix, target_os="macos"))]
mod macos;
#[cfg(all(unix, target_os="macos"))]
use macos::*;

#[cfg(all(unix, not(target_os="macos")))]
mod linux;
#[cfg(all(unix, not(target_os="macos")))]
use linux::*;

const TEENSY_VENDOR_ID: u16 = 0x16C0;
const TEENSY_PRODUCT_ID: u16 = 0x0478;

pub struct Teensy {
    sys: SysTeensy,
    code_size: usize,
    block_size: usize,
    write_size: usize,
}

impl Teensy {
    pub fn connect(code_size: usize, block_size: usize) -> Result<Self, ()> {
        let write_size = if block_size == 512 || block_size == 1024 {
            block_size + 64
        } else {
            block_size + 2
        };

        Ok(Teensy { sys: SysTeensy::connect()?, code_size, block_size, write_size })
    }

    pub fn write(&mut self, buf: &[u8], timeout: Duration) -> Result<(), ()> {
        self.sys.write(buf, timeout)
    }

    pub fn boot(mut self) {
        // FIXME: Verbose
        println!("Booting");

        let mut buf = Vec::<u8>::with_capacity(self.write_size);
        buf.extend(std::iter::repeat(0).take(self.write_size as usize));
        buf[0] = 0xff;
        buf[1] = 0xff;
        buf[2] = 0xff;
        let _ = self.write(&buf, Duration::from_millis(500));
    }
}
