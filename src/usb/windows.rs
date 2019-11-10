use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::usb::*;

use winapi::ctypes::c_void;
//use winapi::shared::hidclass::*;
use winapi::shared::hidsdi::*;
use winapi::shared::minwindef::*;
use winapi::shared::winerror::*;
use winapi::um::errhandlingapi::*;
use winapi::um::fileapi::*;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::*;
use winapi::um::minwinbase::*;
use winapi::um::setupapi::*;
use winapi::um::synchapi::*;
use winapi::um::winbase::*;
use winapi::um::winnt::*;

#[derive(Debug, PartialEq)]
pub enum SystemError {
    CreateHandle,
    IoPending,
    NoBytesWritten,
    OverlapError,
}

pub struct SysTeensy {
    teensy_handle: HANDLE,
    write_event: Option<HANDLE>,
}

impl SysTeensy {
    pub fn connect(vid: u16, pid: u16) -> Result<Self, ConnectError> {
        Ok(SysTeensy {
            teensy_handle: unsafe { open_usb_device(vid, pid)? },
            write_event: None,
        })
    }

    unsafe fn __write(&mut self, buf: &[u8], timeout: u32) -> Result<(), WriteError> {
        if let None = self.write_event {
            let event = CreateEventA(null_mut(), TRUE, TRUE, null());
            if event.is_null() {
                return Err(WriteError::System(SystemError::CreateHandle));
            }
            self.write_event = Some(event);
        }
        let event = self.write_event.unwrap();

        ResetEvent(event);

        let mut ov = OVERLAPPED::default();
        ov.hEvent = event;
        let mut tempbuf = vec![0];
        tempbuf.extend(buf);

        if WriteFile(
            self.teensy_handle,
            tempbuf.as_ptr() as *const c_void,
            tempbuf.len() as DWORD,
            null_mut(),
            &mut ov,
        ) == 0 {
            if GetLastError() != ERROR_IO_PENDING {
                return Err(WriteError::System(SystemError::IoPending));
            }

            let ret = WaitForSingleObject(event, timeout);
            if ret == WAIT_TIMEOUT {
                CancelIo(self.teensy_handle);
                return Err(WriteError::Timeout);
            }
        }

        let mut n = 0;
        if GetOverlappedResult(self.teensy_handle, &mut ov, &mut n, FALSE) == 0 {
            return Err(WriteError::System(SystemError::OverlapError));
        }
        if n <= 0 {
            return Err(WriteError::System(SystemError::NoBytesWritten));
        }

        Ok(())
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
            if let Ok(_) = unsafe {
                self.__write(buf, time_left(begin, timeout).as_millis() as u32)
            } {
                return Ok(());
            }
            sleep(Duration::from_millis(10));
        }
        Err(WriteError::Timeout)
    }
}

impl Drop for SysTeensy {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.teensy_handle);
        }
    }
}

unsafe fn open_usb_device(vid: u16, pid: u16) -> Result<HANDLE, ConnectError> {
    let mut guid = Default::default();
    HidD_GetHidGuid(&mut guid);

    let info = SetupDiGetClassDevsA(
        &guid,
        null(),
        null_mut(),
        DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
    );
    if info == INVALID_HANDLE_VALUE {
        return Err(ConnectError::System(SystemError::CreateHandle));
    }

    let mut index = 0;
    loop {
        let mut iface = SP_DEVICE_INTERFACE_DATA::default();
        iface.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as DWORD;
        if SetupDiEnumDeviceInterfaces(info, null_mut(), &guid, index, &mut iface) == 0  {
            SetupDiDestroyDeviceInfoList(info);
            break;
        }
        index += 1;

        let mut required_size = 0;
        SetupDiGetDeviceInterfaceDetailA(
            info,
            &mut iface,
            null_mut(),
            0,
            &mut required_size,
            null_mut(),
        );

        // malloc `details`
        let mut details_buf = Vec::<u8>::with_capacity(required_size as usize);
        details_buf.resize(required_size as usize, 0);

        let details = details_buf.as_mut_ptr() as PSP_DEVICE_INTERFACE_DETAIL_DATA_A;
        (*details).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_A>() as DWORD;
        if SetupDiGetDeviceInterfaceDetailA(
            info,
            &mut iface,
            details,
            required_size,
            null_mut(),
            null_mut(),
        ) == 0 {
            // free `details`
            Vec::from_raw_parts(
                details as *mut u8,
                required_size as usize,
                required_size as usize,
            );
            continue;
        }

        let h = CreateFileA(
            (*details).DevicePath.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            null_mut(),
        );
        {
            // free `details`
            Vec::from_raw_parts(
                details as *mut u8,
                required_size as usize,
                required_size as usize,
            );
        }

        if h == INVALID_HANDLE_VALUE {
            continue;
        }

        let mut attrib = HIDD_ATTRIBUTES::default();
        attrib.Size = size_of::<HIDD_ATTRIBUTES>() as ULONG;
        if HidD_GetAttributes(h, &mut attrib) == 0 {
            CloseHandle(h);
            continue;
        }
        if attrib.VendorID != vid || attrib.ProductID != pid {
            CloseHandle(h);
            continue;
        }

        SetupDiDestroyDeviceInfoList(info);
        return Ok(h);
    }

    Err(ConnectError::DeviceNotFound)
}
