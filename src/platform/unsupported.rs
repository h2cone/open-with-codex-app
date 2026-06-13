use std::ffi::OsString;
use std::io;
use std::path::Path;

use super::InstallReport;

pub fn install_integrations() -> io::Result<InstallReport> {
    let mut report = InstallReport::default();
    report.skipped("unsupported platform; only Windows and macOS are supported");
    Ok(report)
}

pub fn uninstall_integrations() -> io::Result<InstallReport> {
    let mut report = InstallReport::default();
    report.skipped("unsupported platform; only Windows and macOS are supported");
    Ok(report)
}

pub fn open_project(_path: &Path) -> io::Result<String> {
    Ok("skipped: unsupported platform; only Windows and macOS are supported".to_string())
}

pub fn forward_to_existing_codex(_args: &[OsString]) -> io::Result<i32> {
    Ok(1)
}
