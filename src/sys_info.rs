use std::sync::OnceLock;

#[cfg(target_os = "windows")]
pub fn is_win11() -> bool {
    static IS_WIN11: OnceLock<bool> = OnceLock::new();
    
    *IS_WIN11.get_or_init(|| {
        use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;
        
        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlGetVersion(lpVersionInformation: *mut OSVERSIONINFOW) -> i32;
        }

        let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
        info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        let status = unsafe { RtlGetVersion(&mut info) };
        if status == 0 {
            info.dwMajorVersion >= 10 && info.dwBuildNumber >= 22000
        } else {
            false
        }
    })
}

#[cfg(not(target_os = "windows"))]
pub fn is_win11() -> bool {
    false
}
