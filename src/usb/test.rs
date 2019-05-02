pub struct SysTeensy;

impl SysTeensy {
    pub fn connect() -> Result<Self, ()> {
        unimplemented!()
    }

    pub fn write(self, buf: &[u8]) -> Result<(), ()> {
        unimplemented!()
    }
}

impl Drop for SysTeensy {
    fn drop(&mut self) {
        unimplemented!()
    }
}
