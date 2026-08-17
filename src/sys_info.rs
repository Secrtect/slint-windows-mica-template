//! 系统环境与版本检测模块
//! System environment and OS version detection module.

use std::sync::OnceLock;

/// 检测当前系统是否为 Windows 11 (Build >= 22000)
/// Detect whether current operating system is Windows 11 (Build >= 22000)
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

/// 非 Windows 平台降级实现
/// Non-Windows platform fallback implementation
#[cfg(not(target_os = "windows"))]
pub fn is_win11() -> bool {
    false
}
