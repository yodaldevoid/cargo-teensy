use std::time::Duration;

use crate::Mcu;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

#[cfg(all(unix, target_os="macos"))]
mod macos;
#[cfg(all(unix, target_os="macos"))]
use macos as sys;

#[cfg(all(unix, not(target_os="macos")))]
mod linux;
#[cfg(all(unix, not(target_os="macos")))]
use linux as sys;

const TEENSY_VENDOR_ID: u16 = 0x16C0;
const TEENSY_PRODUCT_ID: u16 = 0x0478;

#[derive(Debug, PartialEq)]
pub enum ConnectError {
    System(sys::SystemError),
    DeviceNotFound,
}

#[derive(Debug, PartialEq)]
pub enum WriteError {
    System(sys::SystemError),
    Timeout,
}

#[derive(Debug, PartialEq)]
pub enum ProgramError {
    BinaryRemainder,
    UnknownBlockSize(usize),
    WriteError(WriteError),
}

impl From<WriteError> for ProgramError {
    fn from(err: WriteError) -> Self {
        ProgramError::WriteError(err)
    }
}

pub struct Teensy {
    sys: sys::SysTeensy,
    code_size: usize,
    block_size: usize,
    header_size: usize,
}

impl Teensy {
    pub fn connect(mcu: Mcu) -> Result<Self, ConnectError> {
        let header_size =
            if mcu.block_size == 512 || mcu.block_size == 1024 { 64 } else { 2 };

        Ok(Self {
            sys: sys::SysTeensy::connect(TEENSY_VENDOR_ID, TEENSY_PRODUCT_ID)?,
            code_size: mcu.code_size,
            block_size: mcu.block_size,
            header_size,
        })
    }

    pub fn write(&mut self, buf: &[u8], timeout: Duration) -> Result<(), WriteError> {
        self.sys.write(buf, timeout)
    }

    pub fn boot(&mut self) -> Result<(), WriteError> {
        let mut buf = Vec::<u8>::with_capacity(self.write_size());
        buf.extend(std::iter::repeat(0).take(self.write_size() as usize));
        buf[0] = 0xff;
        buf[1] = 0xff;
        buf[2] = 0xff;
        self.write(&buf, Duration::from_millis(500))
    }

    pub fn program(
        &mut self,
        binary: &[u8],
        feedback: impl Fn(usize)
    ) -> Result<(), ProgramError> {
        let binary_chunks = binary.chunks_exact(self.block_size);
        if !binary_chunks.remainder().is_empty() {
            return Err(ProgramError::BinaryRemainder);
        }

        let mut buf = Vec::with_capacity(self.write_size());
        for (addr, chunk) in (0..self.code_size).step_by(self.block_size).zip(binary_chunks) {
            if addr != 0 && chunk.iter().all(|&x| x == 0xFF) {
                continue;
            }

            feedback(addr);

            if self.block_size <= 256 {
                buf.resize(2, 0);
                if self.code_size < 0x10000 {
                    buf[0] = addr as u8;
                    buf[1] = (addr >> 8) as u8;
                } else {
                    buf[0] = (addr >> 8) as u8;
                    buf[1] = (addr >> 16) as u8;
                }
                buf.extend_from_slice(chunk);
            } else {
                buf.resize(64, 0);
                buf[0] = addr as u8;
                buf[1] = (addr >> 8) as u8;
                buf[2] = (addr >> 16) as u8;
                buf.extend_from_slice(chunk);
            }

            self.write(&buf, Duration::from_millis(if addr == 0 { 5000 } else { 500 }))?;
        }

        Ok(())
    }

    fn write_size(&self) -> usize {
        self.block_size + self.header_size
    }
}
