//! Windowed launcher binary. `frametrace.exe` stays a console CLI; this
//! variant hides the console and starts the examiner workstation directly.
#![windows_subsystem = "windows"]

fn main() {
    if let Err(error) = frametrace::serve::run(frametrace::serve::ServeOptions {
        case_dir: None,
        port: None,
    }) {
        // No console is attached in windowed mode; surface fatal startup
        // errors through a message box so the examiner is never left with a
        // silently missing window.
        use std::io::Write;
        let log_path = std::env::temp_dir().join("frametrace-app-error.log");
        if let Ok(mut log) = std::fs::File::create(&log_path) {
            let _ = writeln!(log, "{error}");
        }
        let message = std::ffi::CString::new(format!(
            "FrameTrace failed to start: {error}
(log: {})",
            log_path.display()
        ))
        .expect("message is valid UTF-8");
        #[cfg(target_os = "windows")]
        unsafe {
            MessageBoxA(
                std::ptr::null_mut(),
                message.as_ptr() as *const u8,
                c"FrameTrace".as_ptr() as *const u8,
                0x10,
            );
        }
        #[cfg(not(target_os = "windows"))]
        eprintln!("{error}");
    }
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxA(
        hwnd: *mut core::ffi::c_void,
        text: *const u8,
        caption: *const u8,
        utype: u32,
    ) -> i32;
}
