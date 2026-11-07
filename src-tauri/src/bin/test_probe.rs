use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use filechecker_lib::core::scanner::{ScanObserver, ScanPolicy};
use filechecker_lib::platform::windows::win32_fs::get_directory_frn;

fn main() {
    let path = PathBuf::from("D:\\_Project");
    println!("Testing path: {:?}", path);
    println!("Directory exists? {}", path.exists());
    let frn = get_directory_frn(&path);
    println!("get_directory_frn: {:?}", frn);
}
