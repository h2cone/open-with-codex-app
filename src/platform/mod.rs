use std::ffi::OsString;
use std::io;
use std::path::Path;

pub mod macos_assets;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(windows)]
pub mod windows;

#[cfg(not(any(windows, target_os = "macos")))]
mod unsupported;

#[derive(Debug, Default)]
pub struct InstallReport {
    pub lines: Vec<String>,
}

impl InstallReport {
    pub fn installed(&mut self, subject: &str) {
        self.lines.push(format!("installed: {subject}"));
    }

    pub fn removed(&mut self, subject: &str) {
        self.lines.push(format!("removed: {subject}"));
    }

    pub fn skipped(&mut self, subject: &str) {
        self.lines.push(format!("skipped: {subject}"));
    }
}

#[cfg(target_os = "macos")]
pub use macos::{
    forward_to_existing_codex, install_integrations, open_project, uninstall_integrations,
};

#[cfg(windows)]
pub use windows::{
    forward_to_existing_codex, install_integrations, open_project, uninstall_integrations,
};

#[cfg(not(any(windows, target_os = "macos")))]
pub use unsupported::{
    forward_to_existing_codex, install_integrations, open_project, uninstall_integrations,
};

fn canonical_project_dir(path: &Path) -> io::Result<std::path::PathBuf> {
    let canonical = path.canonicalize()?;
    if !canonical.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("not a directory: {}", path.display()),
        ));
    }
    Ok(canonical)
}

fn current_exe_file_name(default_name: &str) -> OsString {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_os_string()))
        .unwrap_or_else(|| OsString::from(default_name))
}
