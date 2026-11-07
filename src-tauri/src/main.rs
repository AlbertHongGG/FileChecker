// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "windows")]
    {
        if !filechecker_lib::platform::is_process_elevated() {
            if filechecker_lib::platform::relaunch_as_admin() {
                std::process::exit(0);
            }
        }
    }

    filechecker_lib::run()
}
