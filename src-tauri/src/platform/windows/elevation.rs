use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Security::{
    AllocateAndInitializeSid, CheckTokenMembership, FreeSid, SID_IDENTIFIER_AUTHORITY,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Check if the current process is running with an elevated Administrator token.
pub fn is_process_elevated() -> bool {
    let mut nt_authority = SID_IDENTIFIER_AUTHORITY {
        Value: [0, 0, 0, 0, 0, 5],
    };
    let mut admin_group = std::ptr::null_mut();

    let success = unsafe {
        AllocateAndInitializeSid(
            &mut nt_authority,
            2,
            0x20,  // SECURITY_BUILTIN_DOMAIN_RID
            0x220, // DOMAIN_ALIAS_RID_ADMINS
            0,
            0,
            0,
            0,
            0,
            0,
            &mut admin_group,
        )
    };

    if success == 0 || admin_group.is_null() {
        return false;
    }

    let mut is_member = 0i32;
    let ok = unsafe { CheckTokenMembership(std::ptr::null_mut(), admin_group, &mut is_member) };

    unsafe {
        FreeSid(admin_group);
    }

    ok != 0 && is_member != 0
}

/// Relaunches the current executable with Administrator privileges using the "runas" verb.
/// Returns true if the elevated child process was successfully spawned.
pub fn relaunch_as_admin() -> bool {
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let exe_str = current_exe.to_string_lossy();
    let wide_exe: Vec<u16> = OsStr::new(&*exe_str)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let verb: Vec<u16> = OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_str = args
        .iter()
        .map(|a| {
            if a.contains(' ') {
                format!("\"{}\"", a)
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let wide_args: Vec<u16> = OsStr::new(&args_str)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let ret = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            wide_exe.as_ptr(),
            if args_str.is_empty() {
                std::ptr::null()
            } else {
                wide_args.as_ptr()
            },
            std::ptr::null(),
            SW_SHOWNORMAL as i32,
        )
    };

    (ret as usize) > 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_process_elevated_callable() {
        // Just verify it calls Windows API without crashing or memory corruption
        let _ = is_process_elevated();
    }
}
