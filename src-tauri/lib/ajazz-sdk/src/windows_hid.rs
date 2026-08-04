use std::{
    ffi::{CStr, c_char, c_int, c_ulong},
    ptr::{NonNull, null_mut},
};

use hidapi::{HidError, HidResult};

const STRING_BUFFER_LENGTH: usize = 128;
const WRITE_TIMEOUT_MS: c_ulong = 5_000;

#[repr(C)]
struct RawHidDevice {
    _private: [u8; 0],
}

type StringGetter = unsafe extern "C" fn(*mut RawHidDevice, *mut u16, usize) -> c_int;

unsafe extern "C" {
    fn hid_open_path(path: *const c_char) -> *mut RawHidDevice;
    fn hid_close(device: *mut RawHidDevice);
    fn hid_error(device: *mut RawHidDevice) -> *const u16;
    fn hid_write(device: *mut RawHidDevice, data: *const u8, length: usize) -> c_int;
    fn hid_read(device: *mut RawHidDevice, data: *mut u8, length: usize) -> c_int;
    fn hid_read_timeout(
        device: *mut RawHidDevice,
        data: *mut u8,
        length: usize,
        milliseconds: c_int,
    ) -> c_int;
    fn hid_set_nonblocking(device: *mut RawHidDevice, nonblock: c_int) -> c_int;
    fn hid_get_feature_report(
        device: *mut RawHidDevice,
        data: *mut u8,
        length: usize,
    ) -> c_int;
    fn hid_get_manufacturer_string(
        device: *mut RawHidDevice,
        value: *mut u16,
        length: usize,
    ) -> c_int;
    fn hid_get_product_string(
        device: *mut RawHidDevice,
        value: *mut u16,
        length: usize,
    ) -> c_int;
    fn hid_get_serial_number_string(
        device: *mut RawHidDevice,
        value: *mut u16,
        length: usize,
    ) -> c_int;
    fn hid_winapi_set_write_timeout(device: *mut RawHidDevice, timeout: c_ulong);
}

/// Windows HID handle with a write timeout suitable for multi-report images.
pub(crate) struct WindowsHidDevice {
    device: NonNull<RawHidDevice>,
}

// This mirrors the Send guarantee of hidapi::HidDevice, which owns the same
// native handle. Ajazz/AsyncAjazz serializes access after moving it.
unsafe impl Send for WindowsHidDevice {}

impl WindowsHidDevice {
    pub(crate) fn open(path: &CStr) -> HidResult<Self> {
        // SAFETY: path is a valid, null-terminated HID path supplied by hidapi.
        let device = unsafe { hid_open_path(path.as_ptr()) };
        let Some(device) = NonNull::new(device) else {
            return Err(last_error(null_mut()));
        };

        // The hidapi Windows backend defaults to one second. A touchscreen
        // image is split into many output reports, and the device can require
        // longer while committing a previous frame.
        unsafe { hid_winapi_set_write_timeout(device.as_ptr(), WRITE_TIMEOUT_MS) };
        Ok(Self { device })
    }

    pub(crate) fn write(&self, data: &[u8]) -> HidResult<usize> {
        if data.is_empty() {
            return Err(HidError::InvalidZeroSizeData);
        }
        // SAFETY: data remains valid for this synchronous call.
        self.size_result(unsafe { hid_write(self.device.as_ptr(), data.as_ptr(), data.len()) })
    }

    pub(crate) fn read(&self, data: &mut [u8]) -> HidResult<usize> {
        // SAFETY: data is a valid writable buffer for this synchronous call.
        self.size_result(unsafe {
            hid_read(self.device.as_ptr(), data.as_mut_ptr(), data.len())
        })
    }

    pub(crate) fn read_timeout(&self, data: &mut [u8], timeout: c_int) -> HidResult<usize> {
        // SAFETY: data is a valid writable buffer for this synchronous call.
        self.size_result(unsafe {
            hid_read_timeout(self.device.as_ptr(), data.as_mut_ptr(), data.len(), timeout)
        })
    }

    pub(crate) fn set_blocking_mode(&self, blocking: bool) -> HidResult<()> {
        // hidapi expresses blocking mode through the inverse nonblocking flag.
        let result =
            unsafe { hid_set_nonblocking(self.device.as_ptr(), if blocking { 0 } else { 1 }) };
        if result == 0 {
            Ok(())
        } else {
            Err(self.error())
        }
    }

    pub(crate) fn get_feature_report(&self, data: &mut [u8]) -> HidResult<usize> {
        // SAFETY: data is a valid writable buffer for this synchronous call.
        self.size_result(unsafe {
            hid_get_feature_report(self.device.as_ptr(), data.as_mut_ptr(), data.len())
        })
    }

    pub(crate) fn get_manufacturer_string(&self) -> HidResult<Option<String>> {
        self.get_string(hid_get_manufacturer_string)
    }

    pub(crate) fn get_product_string(&self) -> HidResult<Option<String>> {
        self.get_string(hid_get_product_string)
    }

    pub(crate) fn get_serial_number_string(&self) -> HidResult<Option<String>> {
        self.get_string(hid_get_serial_number_string)
    }

    fn get_string(&self, getter: StringGetter) -> HidResult<Option<String>> {
        let mut buffer = [0u16; STRING_BUFFER_LENGTH];
        // SAFETY: buffer is valid for STRING_BUFFER_LENGTH UTF-16 code units.
        let result =
            unsafe { getter(self.device.as_ptr(), buffer.as_mut_ptr(), buffer.len()) };
        if result != 0 {
            return Err(self.error());
        }
        let length = buffer
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(buffer.len());
        Ok(Some(String::from_utf16_lossy(&buffer[..length])))
    }

    fn size_result(&self, result: c_int) -> HidResult<usize> {
        if result < 0 {
            Err(self.error())
        } else {
            Ok(result as usize)
        }
    }

    fn error(&self) -> HidError {
        last_error(self.device.as_ptr())
    }
}

impl Drop for WindowsHidDevice {
    fn drop(&mut self) {
        // SAFETY: this pointer was returned by hid_open_path and is owned here.
        unsafe { hid_close(self.device.as_ptr()) };
    }
}

fn last_error(device: *mut RawHidDevice) -> HidError {
    // SAFETY: hid_error accepts either a live device pointer or null for the
    // last global open error, and returns a null-terminated UTF-16 string.
    let message = unsafe { hid_error(device) };
    if message.is_null() {
        return HidError::HidApiErrorEmpty;
    }

    let mut length = 0;
    // SAFETY: hidapi owns a null-terminated error string for the duration of
    // this call. We copy it before another hidapi operation can replace it.
    unsafe {
        while *message.add(length) != 0 {
            length += 1;
        }
        HidError::HidApiError {
            message: String::from_utf16_lossy(std::slice::from_raw_parts(message, length)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WRITE_TIMEOUT_MS;

    #[test]
    fn image_write_timeout_exceeds_the_hidapi_default() {
        assert_eq!(WRITE_TIMEOUT_MS, 5_000);
    }
}
