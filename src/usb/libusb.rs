use std::thread::sleep;
use std::time::{Duration, Instant};

use rusb::{GlobalContext, DeviceHandle, UsbContext};

use crate::usb::*;

#[derive(Debug, PartialEq)]
pub enum SystemError {
    LibUsb(rusb::Error),
}

impl From<rusb::Error> for SystemError {
    // FIXME: separate out into different errors
    fn from(err: rusb::Error) -> Self {
        SystemError::LibUsb(err)
    }
}

impl From<rusb::Error> for ConnectError {
    fn from(err: rusb::Error) -> Self {
        ConnectError::System(err.into())
    }
}

pub struct SysTeensy {
    teensy_handle: DeviceHandle<GlobalContext>,
}

impl SysTeensy {
    pub fn connect(vid: u16, pid: u16) -> Result<Self, ConnectError> {
        let mut context = GlobalContext {};
        let mut device = open_usb_device(&mut context, vid, pid)?;
        match device.kernel_driver_active(0) {
            Ok(true) => {
                device.detach_kernel_driver(0)?;
            }
            Ok(false) | Err(rusb::Error::NotSupported) => {}
            Err(err) => return Err(ConnectError::System(SystemError::LibUsb(err))),
        }

        device.claim_interface(0)?;

        Ok(SysTeensy { teensy_handle: device })
    }

    pub fn write(&mut self, buf: &[u8], timeout: Duration) -> Result<(), WriteError> {
        fn time_left(begin: Instant, timeout: Duration) -> Duration {
            let passed = begin.elapsed();
            if passed < timeout {
                timeout - passed
            } else {
                Duration::new(0, 0)
            }
        }

        let begin = Instant::now();
        while begin.elapsed() < timeout {
            let num_written = match self.teensy_handle.write_control(
                0x21,
                9,
                0x0200,
                0,
                buf,
                time_left(begin, timeout)
            ) {
                Ok(n) => n,
                Err(rusb::Error::Timeout) => 0,
                Err(err) => return Err(WriteError::System(SystemError::LibUsb(err))),
            };

            if num_written >= buf.len() {
                return Ok(())
            }
            sleep(Duration::from_millis(10));
        }
        Err(WriteError::Timeout)
    }
}

fn open_usb_device<C: UsbContext>(
    context: &mut C,
    vid: u16,
    pid: u16,
) -> Result<DeviceHandle<C>, ConnectError> {
    for device in context.devices()?.iter() {
        let desc = device.device_descriptor()?;

        if desc.vendor_id() == vid && desc.product_id() == pid {
            return Ok(device.open()?);
        }
    }

    Err(ConnectError::DeviceNotFound)
}
