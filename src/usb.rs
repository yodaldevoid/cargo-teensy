use std::time::Duration;

use crate::Mcu;

#[cfg(all(windows, not(feature="libusb")))]
mod windows;
#[cfg(all(windows, not(feature="libusb")))]
use windows as sys;

#[cfg(all(all(unix, target_os="macos"), not(feature="libusb")))]
mod macos;
#[cfg(all(all(unix, target_os="macos"), not(feature="libusb")))]
use macos as sys;

#[cfg(any(all(unix, not(target_os="macos")), feature="libusb"))]
mod libusb;
#[cfg(any(all(unix, not(target_os="macos")), feature="libusb"))]
use libusb as sys;

const TEENSY_VENDOR_ID: u16 = 0x16C0;
const TEENSY_PRODUCT_ID: u16 = 0x0478;
const SOFT_REBOOTER_PRODUCT_ID: u16 = 0x0483;

#[derive(Debug, PartialEq)]
pub enum ConnectError {
    System(sys::SystemError),
    DeviceNotFound,
}

impl From<sys::SystemError> for ConnectError {
    fn from(err: sys::SystemError) -> Self {
        ConnectError::System(err)
    }
}

#[derive(Debug, PartialEq)]
pub enum WriteError {
    System(sys::SystemError),
    Timeout,
}

impl From<sys::SystemError> for WriteError {
    fn from(err: sys::SystemError) -> Self {
        WriteError::System(err)
    }
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

pub struct SoftRebootor {
    sys: sys::SysTeensy,
}

impl SoftRebootor {
    pub fn connect() -> Result<Self, ConnectError> {
        Ok(Self {
            sys: sys::SysTeensy::connect(TEENSY_VENDOR_ID, SOFT_REBOOTER_PRODUCT_ID)?,
        })
    }

    pub fn reboot(&mut self) -> Result<(), WriteError> {
        unimplemented!()
        /*
        request_type: 0x21, // Request type: host to device, class, interface
        request: 0x20, // Request: CDC set line coding
        value: 0, // Value: n/a
        index: 0, // Index: interface 0
        length: 1,
        data: 134,
        */
        //let buf = [134];
        //self.sys.write(&buf, Duration::from_millis(500))
    }
}
