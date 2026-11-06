#[cfg(target_os = "windows")]
pub fn reveal_in_explorer(full_path: &str) {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let mut cmd = Command::new("explorer");
    // DETACHED_PROCESS flag to prevent console window popup
    cmd.creation_flags(0x08000000);
    cmd.args(["/select,", full_path]);
    let _ = cmd.spawn();
}

#[cfg(not(target_os = "windows"))]
pub fn reveal_in_explorer(_full_path: &str) {
    // Unsupported on other platforms
}
