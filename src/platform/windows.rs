use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_SZ};
use winreg::{RegKey, RegValue};

use super::{canonical_project_dir, current_exe_file_name, InstallReport};

const MENU_KEY: &str = r"Software\Classes\Directory\shell\OpenWithCodexApp";
const BACKGROUND_MENU_KEY: &str = r"Software\Classes\Directory\Background\shell\OpenWithCodexApp";
const ENVIRONMENT_KEY: &str = r"Environment";

pub fn install_integrations() -> io::Result<InstallReport> {
    let mut report = InstallReport::default();

    if find_codex_app().is_none() {
        report.skipped("Codex app not installed");
        return Ok(report);
    }

    let launcher = stable_launcher_path()?;
    if copy_current_exe_if_different(&launcher)? {
        report.installed("stable Open with Codex app launcher");
    } else {
        report.skipped("stable Open with Codex app launcher already exists");
    }

    if context_menu_registered()? {
        report.skipped("Open with Codex app Explorer context menu already registered");
    } else {
        register_context_menu(&launcher)?;
        report.installed("Open with Codex app Explorer context menu");
    }

    let cli = cli_shim_path()?;
    if copy_current_exe_if_different(&cli)? {
        report.installed("codex app command shim");
    } else {
        report.skipped("codex app command shim already registered");
    }

    let cli_dir = cli.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "CLI shim path has no parent")
    })?;
    if user_path_contains(cli_dir)? {
        report.skipped("codex app command shim directory already on user PATH");
    } else {
        prepend_user_path(cli_dir)?;
        report.installed("codex app command shim directory on user PATH");
    }

    Ok(report)
}

pub fn uninstall_integrations() -> io::Result<InstallReport> {
    let mut report = InstallReport::default();

    remove_context_menu(&mut report)?;
    remove_cli_path(&mut report)?;
    remove_integration_files(&mut report)?;

    Ok(report)
}

pub fn open_project(path: &Path) -> io::Result<String> {
    let project = strip_windows_verbatim_prefix(&canonical_project_dir(path)?);
    let Some(codex) = find_codex_app() else {
        return Ok("skipped: Codex app not installed".to_string());
    };

    match spawn_codex(&codex, &project) {
        Ok(()) => Ok(format!("opened: {}", project.display())),
        Err(first_error) => {
            if let Some(alias) = windows_app_execution_alias() {
                if alias != codex && spawn_codex(&alias, &project).is_ok() {
                    return Ok(format!("opened: {}", project.display()));
                }
            }
            Err(first_error)
        }
    }
}

pub fn forward_to_existing_codex(args: &[OsString]) -> io::Result<i32> {
    let Some(target) = find_forward_target()? else {
        eprintln!("codex app shim is installed, but no existing codex CLI was found to forward this command.");
        return Ok(1);
    };

    run_forward_target(&target, args)
}

pub fn context_menu_command(launcher: &Path, folder_arg: &str) -> String {
    format!(r#""{}" app "{}""#, launcher.display(), folder_arg)
}

pub fn strip_windows_verbatim_prefix(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();

    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }

    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }

    path.to_path_buf()
}

fn register_context_menu(launcher: &Path) -> io::Result<()> {
    write_menu_key(MENU_KEY, &context_menu_command(launcher, "%1"))?;
    write_menu_key(BACKGROUND_MENU_KEY, &context_menu_command(launcher, "%V"))?;
    Ok(())
}

fn remove_context_menu(report: &mut InstallReport) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    remove_registry_tree(
        &hkcu,
        MENU_KEY,
        "Open with Codex app folder context menu",
        report,
    )?;
    remove_registry_tree(
        &hkcu,
        BACKGROUND_MENU_KEY,
        "Open with Codex app folder background context menu",
        report,
    )?;
    Ok(())
}

fn remove_registry_tree(
    root: &RegKey,
    path: &str,
    subject: &str,
    report: &mut InstallReport,
) -> io::Result<()> {
    match root.delete_subkey_all(path) {
        Ok(()) => report.removed(subject),
        Err(err) if err.kind() == io::ErrorKind::NotFound => report.skipped(subject),
        Err(err) => return Err(err),
    }
    Ok(())
}

fn write_menu_key(path: &str, command: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(path)?;
    key.set_value("", &"Open with Codex app")?;
    key.set_value("Icon", &stable_launcher_path()?.to_string_lossy().as_ref())?;
    let (command_key, _) = key.create_subkey("command")?;
    command_key.set_value("", &command)?;
    Ok(())
}

fn context_menu_registered() -> io::Result<bool> {
    Ok(registry_default_value_exists(MENU_KEY, "command")?
        && registry_default_value_exists(BACKGROUND_MENU_KEY, "command")?)
}

fn registry_default_value_exists(key_path: &str, subkey: &str) -> io::Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey_with_flags(format!(r"{key_path}\{subkey}"), KEY_READ) else {
        return Ok(false);
    };
    let value: io::Result<String> = key.get_value("");
    Ok(value.map(|value| !value.is_empty()).unwrap_or(false))
}

fn copy_current_exe_if_different(destination: &Path) -> io::Result<bool> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let current = env::current_exe()?;
    copy_file_if_different(&current, destination)
}

fn copy_file_if_different(source: &Path, destination: &Path) -> io::Result<bool> {
    if path_eq_ignore_case(source, destination) {
        return Ok(false);
    }

    if destination.exists() && files_have_same_contents(source, destination)? {
        return Ok(false);
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(true)
}

fn files_have_same_contents(left: &Path, right: &Path) -> io::Result<bool> {
    let left_metadata = fs::metadata(left)?;
    let right_metadata = fs::metadata(right)?;
    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }

    Ok(fs::read(left)? == fs::read(right)?)
}

fn stable_launcher_path() -> io::Result<PathBuf> {
    Ok(local_app_data()?
        .join("OpenAI")
        .join("Codex")
        .join("OpenWithCodexApp")
        .join(current_exe_file_name("open-with-codex-app.exe")))
}

fn cli_shim_path() -> io::Result<PathBuf> {
    Ok(cli_shim_path_for_local_app_data(&local_app_data()?))
}

pub fn cli_shim_path_for_local_app_data(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("OpenAI")
        .join("Codex")
        .join("OpenWithCodexApp")
        .join("bin")
        .join("codex.exe")
}

fn local_app_data() -> io::Result<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is not set"))
}

fn find_codex_app() -> Option<PathBuf> {
    env_path("CODEX_APP_EXECUTABLE")
        .filter(|path| path.is_file())
        .or_else(scan_windows_apps_for_codex)
        .or_else(find_codex_with_powershell)
        .or_else(windows_app_execution_alias)
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn windows_app_execution_alias() -> Option<PathBuf> {
    let aliases = local_app_data().ok()?.join("Microsoft").join("WindowsApps");
    ["ChatGPT.exe", "Codex.exe"]
        .into_iter()
        .map(|name| aliases.join(name))
        .find(|path| path.is_file())
}

fn scan_windows_apps_for_codex() -> Option<PathBuf> {
    let root = env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
        .join("WindowsApps");
    let mut candidates = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("OpenAI.Codex_")
                .then(|| find_app_executable_in_package(&entry.path()))
        })
        .flatten()
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop()
}

fn find_codex_with_powershell() -> Option<PathBuf> {
    let script = r#"$pkg = Get-AppxPackage -Name OpenAI.Codex -ErrorAction SilentlyContinue | Sort-Object Version -Descending | Select-Object -First 1; if ($pkg) { @('app\ChatGPT.exe', 'app\Codex.exe') | ForEach-Object { $candidate = Join-Path $pkg.InstallLocation $_; if (Test-Path -LiteralPath $candidate -PathType Leaf) { $candidate; break } } }"#;
    let output = Command::new(system_powershell())
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

fn find_app_executable_in_package(package: &Path) -> Option<PathBuf> {
    ["ChatGPT.exe", "Codex.exe"]
        .into_iter()
        .map(|name| package.join("app").join(name))
        .find(|path| path.is_file())
}

fn spawn_codex(codex: &Path, project: &Path) -> io::Result<()> {
    Command::new(codex)
        .arg("--open-project")
        .arg(project)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

fn user_path_contains(dir: &Path) -> io::Result<bool> {
    let path = user_path_value()?;
    let contains = split_windows_path(&path).any(|entry| path_eq_ignore_case(&entry, dir));
    Ok(contains)
}

fn remove_cli_path(report: &mut InstallReport) -> io::Result<()> {
    let cli = cli_shim_path()?;
    let cli_dir = cli.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "CLI shim path has no parent")
    })?;
    let old_path = user_path_value()?;
    let new_path = remove_path_entry_from_value(&old_path, cli_dir);

    if new_path == old_path {
        report.skipped("codex app command shim directory on user PATH");
        return Ok(());
    }

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (environment, _) = hkcu.create_subkey_with_flags(ENVIRONMENT_KEY, KEY_WRITE)?;
    let value_type = environment
        .get_raw_value("Path")
        .map(|value| match value.vtype {
            REG_SZ | REG_EXPAND_SZ => value.vtype,
            _ => REG_EXPAND_SZ,
        })
        .unwrap_or(REG_EXPAND_SZ);
    environment.set_raw_value(
        "Path",
        &RegValue {
            vtype: value_type,
            bytes: encode_registry_string(&new_path),
        },
    )?;

    if let Some(process_path) = env::var_os("PATH") {
        let process_path = process_path.to_string_lossy();
        env::set_var("PATH", remove_path_entry_from_value(&process_path, cli_dir));
    }

    report.removed("codex app command shim directory from user PATH");
    Ok(())
}

fn prepend_user_path(dir: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (environment, _) = hkcu.create_subkey_with_flags(ENVIRONMENT_KEY, KEY_WRITE)?;
    let old_path = user_path_value()?;
    let dir_text = dir.to_string_lossy();
    let new_path = if old_path.trim().is_empty() {
        dir_text.to_string()
    } else {
        format!("{dir_text};{old_path}")
    };
    let value_type = environment
        .get_raw_value("Path")
        .map(|value| match value.vtype {
            REG_SZ | REG_EXPAND_SZ => value.vtype,
            _ => REG_EXPAND_SZ,
        })
        .unwrap_or(REG_EXPAND_SZ);
    environment.set_raw_value(
        "Path",
        &RegValue {
            vtype: value_type,
            bytes: encode_registry_string(&new_path),
        },
    )?;

    if let Some(process_path) = env::var_os("PATH") {
        let process_path = process_path.to_string_lossy();
        env::set_var("PATH", format!("{dir_text};{process_path}"));
    }

    Ok(())
}

pub fn remove_path_entry_from_value(value: &str, dir: &Path) -> String {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.trim_matches('"'))
        .filter(|entry| !path_eq_ignore_case(Path::new(entry), dir))
        .collect::<Vec<_>>()
        .join(";")
}

fn encode_registry_string(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn user_path_value() -> io::Result<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(environment) = hkcu.open_subkey_with_flags(ENVIRONMENT_KEY, KEY_READ) else {
        return Ok(String::new());
    };
    Ok(environment.get_value("Path").unwrap_or_default())
}

fn split_windows_path(value: &str) -> impl Iterator<Item = PathBuf> + '_ {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.trim_matches('"'))
        .map(PathBuf::from)
}

fn path_eq_ignore_case(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.to_string_lossy().trim_end_matches(['\\', '/']))
}

fn find_forward_target() -> io::Result<Option<ForwardTarget>> {
    let current = env::current_exe().ok();
    let current_dir = current
        .as_ref()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf);
    let Some(path_var) = env::var_os("PATH") else {
        return Ok(None);
    };

    for dir in env::split_paths(&path_var) {
        if current_dir
            .as_ref()
            .is_some_and(|current_dir| path_eq_ignore_case(current_dir, &dir))
        {
            continue;
        }

        for target in [
            ForwardTarget::Direct(dir.join("codex.exe")),
            ForwardTarget::Batch(dir.join("codex.cmd")),
            ForwardTarget::Batch(dir.join("codex.bat")),
        ] {
            if target.path().is_file()
                && current
                    .as_ref()
                    .is_none_or(|current| !path_eq_ignore_case(current, target.path()))
            {
                return Ok(Some(target));
            }
        }
    }

    Ok(None)
}

#[derive(Debug)]
enum ForwardTarget {
    Direct(PathBuf),
    Batch(PathBuf),
}

impl ForwardTarget {
    fn path(&self) -> &Path {
        match self {
            ForwardTarget::Direct(path) | ForwardTarget::Batch(path) => path,
        }
    }
}

fn run_forward_target(target: &ForwardTarget, args: &[OsString]) -> io::Result<i32> {
    let status = match target {
        ForwardTarget::Direct(path) => Command::new(path).args(args).status()?,
        ForwardTarget::Batch(path) => Command::new("cmd")
            .arg("/C")
            .arg(path)
            .args(args)
            .status()?,
    };
    Ok(status.code().unwrap_or(1))
}

fn remove_integration_files(report: &mut InstallReport) -> io::Result<()> {
    let root = integration_root()?;
    if !root.exists() {
        report.skipped("Open with Codex app helper files");
        return Ok(());
    }

    let current = env::current_exe().ok();
    if current
        .as_ref()
        .is_some_and(|current| path_is_inside(current, &root))
    {
        remove_file_if_not_running(
            &cli_shim_path()?,
            current.as_ref(),
            "codex app command shim",
            report,
        )?;
        remove_file_if_not_running(
            &stable_launcher_path()?,
            current.as_ref(),
            "stable Open with Codex app launcher",
            report,
        )?;
        report.skipped(
            "Open with Codex app helper directory because the uninstaller is running from it",
        );
        return Ok(());
    }

    fs::remove_dir_all(root)?;
    report.removed("Open with Codex app helper files");
    Ok(())
}

fn remove_file_if_not_running(
    path: &Path,
    current: Option<&PathBuf>,
    subject: &str,
    report: &mut InstallReport,
) -> io::Result<()> {
    if !path.exists() {
        report.skipped(subject);
        return Ok(());
    }

    if current.is_some_and(|current| path_eq_ignore_case(current, path)) {
        report.skipped(subject);
        return Ok(());
    }

    fs::remove_file(path)?;
    report.removed(subject);
    Ok(())
}

fn integration_root() -> io::Result<PathBuf> {
    Ok(local_app_data()?
        .join("OpenAI")
        .join("Codex")
        .join("OpenWithCodexApp"))
}

fn path_is_inside(path: &Path, parent: &Path) -> bool {
    let path = path.to_string_lossy();
    let parent = parent.to_string_lossy();
    path.trim_end_matches(['\\', '/'])
        .to_ascii_lowercase()
        .starts_with(&parent.trim_end_matches(['\\', '/']).to_ascii_lowercase())
}

fn system_powershell() -> PathBuf {
    env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{copy_file_if_different, find_app_executable_in_package};

    #[test]
    fn copy_file_if_different_replaces_existing_foreign_file() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.exe");
        let destination = temp.path().join("codex.exe");
        fs::write(&source, b"our shim").unwrap();
        fs::write(&destination, b"official codex cli").unwrap();

        let copied = copy_file_if_different(&source, &destination).unwrap();

        assert!(copied);
        assert_eq!(fs::read(&destination).unwrap(), b"our shim");
    }

    #[test]
    fn copy_file_if_different_skips_matching_file() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.exe");
        let destination = temp.path().join("codex.exe");
        fs::write(&source, b"same binary").unwrap();
        fs::write(&destination, b"same binary").unwrap();

        let copied = copy_file_if_different(&source, &destination).unwrap();

        assert!(!copied);
        assert_eq!(fs::read(&destination).unwrap(), b"same binary");
    }

    #[test]
    fn package_resolution_prefers_chatgpt_executable() {
        let temp = tempfile::tempdir().unwrap();
        let app = temp.path().join("app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("Codex.exe"), b"legacy executable").unwrap();
        fs::write(app.join("ChatGPT.exe"), b"current executable").unwrap();

        assert_eq!(
            find_app_executable_in_package(temp.path()),
            Some(app.join("ChatGPT.exe"))
        );
    }

    #[test]
    fn package_resolution_falls_back_to_legacy_codex_executable() {
        let temp = tempfile::tempdir().unwrap();
        let app = temp.path().join("app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("Codex.exe"), b"legacy executable").unwrap();

        assert_eq!(
            find_app_executable_in_package(temp.path()),
            Some(app.join("Codex.exe"))
        );
    }
}
