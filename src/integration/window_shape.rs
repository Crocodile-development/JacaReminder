// Gives a borderless window its shape: rounded corners (each one can be rounded or square) and the
// part of it that is inside the screen. A window region does this without needing a transparent
// background (which left light pixels in the corners).

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, RGN_AND, RGN_OR, SetWindowRgn,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowRect, SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA, SystemParametersInfoW,
};

/// The corners of the window.
pub struct Shape {
    /// Radius of the rounded corners, in pixels.
    pub radius: f32,
    /// Which corners are rounded: top left, top right, bottom right, bottom left.
    pub rounded: [bool; 4],
}

/// The main screen, in pixels.
pub struct Screen {
    /// The whole screen: `[left, top, right, bottom]`.
    pub full: [i32; 4],
    /// The screen without the taskbar.
    pub work: [i32; 4],
}

pub fn primary_screen() -> Screen {
    // SAFETY: `work` is filled in by Windows; GetSystemMetrics only reads a number.
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        let mut work: RECT = std::mem::zeroed();
        let known = SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut _, 0) != 0;
        let work = if known { [work.left, work.top, work.right, work.bottom] } else { [0, 0, width, height] };
        Screen { full: [0, 0, width, height], work }
    }
}

/// Applies `shape`. `on_screen` is the part of the screen where the window may show, as
/// `[left, top, right, bottom]` in screen pixels; the rest of the window is cut away (used while
/// the popup slides in from beyond the edge of the monitor, so it never shows on the monitor next
/// to it). `None` keeps the whole window. Does nothing if the window handle is not available.
pub fn apply(window: &impl HasWindowHandle, shape: &Shape, on_screen: Option<[i32; 4]>) {
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = win32.hwnd.get() as HWND;

    // SAFETY: `hwnd` is the live window of this process; `rect` is filled in by Windows. The
    // temporary regions are freed here; the final one is owned by the window after `SetWindowRgn`
    // succeeds, so it must not be freed.
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return;
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let r = shape.radius.round().max(0.0) as i32;
        // The last two numbers are the width and height of the ellipse of a corner. Starting at
        // row 1 leaves out the thin light line that Windows draws at the top of borderless windows.
        let region = CreateRoundRectRgn(0, 1, width + 1, height + 1, r * 2, r * 2);

        // A corner that is not rounded gets its square back.
        let squares = [
            (0, 1, r, 1 + r),
            (width + 1 - r, 1, width + 1, 1 + r),
            (width + 1 - r, height + 1 - r, width + 1, height + 1),
            (0, height + 1 - r, r, height + 1),
        ];
        for (rounded, (left, top, right, bottom)) in shape.rounded.iter().zip(squares) {
            if !rounded {
                let square = CreateRectRgn(left, top, right, bottom);
                CombineRgn(region, region, square, RGN_OR);
                DeleteObject(square);
            }
        }

        if let Some(screen) = on_screen {
            // From screen pixels to window pixels, using where the window really is now.
            let left = (screen[0] - rect.left).max(0);
            let top = (screen[1] - rect.top).max(0);
            let right = (screen[2] - rect.left).min(width);
            let bottom = (screen[3] - rect.top).min(height);
            let clip = if right > left && bottom > top {
                CreateRectRgn(left, top, right, bottom)
            } else {
                CreateRectRgn(0, 0, 0, 0)
            };
            CombineRgn(region, region, clip, RGN_AND);
            DeleteObject(clip);
        }

        SetWindowRgn(hwnd, region, 1);
    }
}
