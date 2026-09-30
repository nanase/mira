//! Windows 固有の問い合わせ。

/// トレイアイコンの大きさ（ピクセル）。表示倍率 100% で 16、150% で 24 になる。
pub fn tray_icon_size() -> u32 {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
        use windows_sys::Win32::UI::WindowsAndMessaging::SM_CXSMICON;
        let size = GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem());
        if size > 0 {
            return size as u32;
        }
    }
    16
}

/// タスクバーが暗い配色か。Windows の「既定の Windows モード」に従う。
pub fn taskbar_is_dark() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
        };
        let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
            .encode_utf16()
            .collect();
        let name: Vec<u16> = "SystemUsesLightTheme\0".encode_utf16().collect();
        let mut value: u32 = 0;
        let mut len = std::mem::size_of::<u32>() as u32;
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut len,
        );
        if status == 0 {
            return value == 0;
        }
    }
    true
}
