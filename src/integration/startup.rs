// The "start with Windows" option: a value in the Run key of the current user.

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW, RegOpenKeyExW,
    RegSetValueExW,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "JacaReminder";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// Turns the option on or off. Returns false if Windows refused.
pub fn set(enabled: bool) -> bool {
    let key_path = wide(RUN_KEY);
    let name = wide(VALUE_NAME);
    let mut key: HKEY = std::ptr::null_mut();

    // SAFETY: all pointers point to live, zero-terminated buffers; the key is closed below.
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_SET_VALUE, &mut key)
            != ERROR_SUCCESS
        {
            return false;
        }
        let ok = if enabled {
            let Ok(exe) = std::env::current_exe() else {
                RegCloseKey(key);
                return false;
            };
            let command = wide(&format!("\"{}\" --daemon", exe.display()));
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr() as *const u8,
                (command.len() * 2) as u32,
            ) == ERROR_SUCCESS
        } else {
            // Deleting a value that is not there is not a problem.
            RegDeleteValueW(key, name.as_ptr());
            true
        };
        RegCloseKey(key);
        ok
    }
}
