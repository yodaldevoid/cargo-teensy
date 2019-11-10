use std::time::Duration;

use crate::usb::*;

pub struct SysTeensy;

impl SysTeensy {
    pub fn connect(vid: u16, pid: u16) -> Result<Self, ConnectError> {
        unimplemented!()
    }

    pub fn write(&mut self, buf: &[u8], timeout: Duration) -> Result<(), WriteError> {
        unimplemented!()
    }
}

impl Drop for SysTeensy {
    fn drop(&mut self) {
        unimplemented!()
    }
}
