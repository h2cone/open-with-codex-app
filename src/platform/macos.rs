use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::macos_assets::{document_wflow_xml, info_plist_xml};
use super::{canonical_project_dir, current_exe_file_name, InstallReport};

const PROFILE_MARKER: &str = "# Added by Open with Codex app";

pub fn install_integrations() -> io::Result<InstallReport> {
    let mut report = InstallReport::default();

    if find_codex_app().is_none() {
        report.skipped("Codex app not installed");
        return Ok(report);
    }

    let launcher = stable_launcher_path()?;
    if copy_current_exe_if_missing(&launcher)? {
        make_executable(&launcher)?;
        report.installed("stable Open with Codex app launcher");
    } else {
        report.skipped("stable Open with Codex app launcher already exists");
    }

    if finder_workflow_registered()? {
        report.skipped("Open with Codex app Finder service already registered");
    } else {
        register_finder_workflow(&launcher)?;
        report.installed("Open with Codex app Finder service");
    }

    let shim = cli_shim_path()?;
    if copy_current_exe_if_missing(&shim)? {
        make_executable(&shim)?;
        report.installed("codex app command shim");
    } else {
        report.skipped("codex app command shim already registered");
    }

    let shim_dir = shim.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "CLI shim path has no parent")
    })?;
    if shell_path_registered(shim_dir)? {
        report.skipped("codex app command shim directory already on shell PATH");
    } else {
        register_shell_path(shim_dir)?;
        report.installed("codex app command shim directory in shell profiles");
    }

    Ok(report)
}

pub fn uninstall_integrations() -> io::Result<InstallReport> {
    let mut report = InstallReport::default();

    remove_finder_workflow(&mut report)?;
    remove_shell_path(&mut report)?;
    remove_owned_cli_shim(&mut report)?;
    remove_integration_files(&mut report)?;

    Ok(report)
}

pub fn open_project(path: &Path) -> io::Result<String> {
    let project = canonical_project_dir(path)?;
    let Some(app) = find_codex_app() else {
        return Ok("skipped: Codex app not installed".to_string());
    };

    Command::new("open")
        .arg("-n")
        .arg(app)
        .arg("--args")
        .arg("--open-project")
        .arg(&project)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(format!("opened: {}", project.display()))
}

pub fn forward_to_existing_codex(args: &[OsString]) -> io::Result<i32> {
    let Some(target) = find_forward_target()? else {
        eprintln!("codex app shim is installed, but no existing codex CLI was found to forward this command.");
        return Ok(1);
    };
    let status = Command::new(target).args(args).status()?;
    Ok(status.code().unwrap_or(1))
}

fn register_finder_workflow(launcher: &Path) -> io::Result<()> {
    let workflow = finder_workflow_path()?;
    let contents = workflow.join("Contents");
    fs::create_dir_all(&contents)?;
    let launcher_text = launcher.to_string_lossy();
    fs::write(contents.join("Info.plist"), info_plist_xml())?;
    fs::write(
        contents.join("document.wflow"),
        document_wflow_xml(&launcher_text),
    )?;
    Ok(())
}

fn remove_finder_workflow(report: &mut InstallReport) -> io::Result<()> {
    let workflow = finder_workflow_path()?;
    if workflow.exists() {
        fs::remove_dir_all(workflow)?;
        report.removed("Open with Codex app Finder service");
    } else {
        report.skipped("Open with Codex app Finder service");
    }
    Ok(())
}

fn finder_workflow_registered() -> io::Result<bool> {
    Ok(finder_workflow_path()?.exists())
}

fn copy_current_exe_if_missing(destination: &Path) -> io::Result<bool> {
    if destination.exists() {
        return Ok(false);
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let current = env::current_exe()?;
    if current == destination {
        return Ok(false);
    }

    fs::copy(current, destination)?;
    Ok(true)
}

fn remove_owned_cli_shim(report: &mut InstallReport) -> io::Result<()> {
    let shim = cli_shim_path()?;
    if !shim.exists() {
        report.skipped("codex app command shim");
        return Ok(());
    }

    let stable = stable_launcher_path()?;
    let current = env::current_exe().ok();
    let owned = stable.exists() && files_have_same_contents(&shim, &stable).unwrap_or(false)
        || current
            .as_ref()
            .is_some_and(|current| files_have_same_contents(&shim, current).unwrap_or(false));

    if owned {
        fs::remove_file(shim)?;
        report.removed("codex app command shim");
    } else {
        report.skipped("codex app command shim because it does not look owned by this tool");
    }

    Ok(())
}

fn files_have_same_contents(left: &Path, right: &Path) -> io::Result<bool> {
    let left_metadata = fs::metadata(left)?;
    let right_metadata = fs::metadata(right)?;
    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }

    Ok(fs::read(left)? == fs::read(right)?)
}

fn make_executable(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

fn stable_launcher_path() -> io::Result<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("Application Support")
        .join("OpenAI")
        .join("Codex")
        .join("OpenWithCodexApp")
        .join(current_exe_file_name("open-with-codex-app")))
}

fn integration_root() -> io::Result<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("Application Support")
        .join("OpenAI")
        .join("Codex")
        .join("OpenWithCodexApp"))
}

fn cli_shim_path() -> io::Result<PathBuf> {
    Ok(home_dir()?.join(".codex").join("bin").join("codex"))
}

fn finder_workflow_path() -> io::Result<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("Services")
        .join("Open with Codex app.workflow"))
}

fn home_dir() -> io::Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))
}

fn find_codex_app() -> Option<PathBuf> {
    env_path("CODEX_APP_BUNDLE")
        .filter(|path| path.exists())
        .or_else(find_standard_codex_bundle)
        .or_else(find_codex_with_mdfind)
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn find_standard_codex_bundle() -> Option<PathBuf> {
    let home = home_dir().ok();
    [
        Some(PathBuf::from("/Applications/Codex.app")),
        Some(PathBuf::from("/Applications/OpenAI Codex.app")),
        home.as_ref()
            .map(|home| home.join("Applications").join("Codex.app")),
        home.as_ref()
            .map(|home| home.join("Applications").join("OpenAI Codex.app")),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.exists())
}

fn find_codex_with_mdfind() -> Option<PathBuf> {
    let output = Command::new("mdfind")
        .arg("kMDItemCFBundleIdentifier == 'com.openai.codex'")
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
        .find(|path| path.exists())
}

fn shell_path_registered(shim_dir: &Path) -> io::Result<bool> {
    if env::var_os("PATH")
        .is_some_and(|value| env::split_paths(&value).any(|path| path == shim_dir))
    {
        return Ok(true);
    }

    let marker = profile_path_block(shim_dir);
    Ok(shell_profiles()?
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .any(|content| content.contains(&marker)))
}

fn remove_shell_path(report: &mut InstallReport) -> io::Result<()> {
    let shim = cli_shim_path()?;
    let shim_dir = shim.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "CLI shim path has no parent")
    })?;
    let mut removed = false;

    for profile in shell_profiles()? {
        let Ok(content) = fs::read_to_string(&profile) else {
            continue;
        };
        let new_content = remove_profile_path_block(&content, shim_dir);
        if new_content != content {
            fs::write(profile, new_content)?;
            removed = true;
        }
    }

    if removed {
        report.removed("codex app command shim directory from shell profiles");
    } else {
        report.skipped("codex app command shim directory in shell profiles");
    }
    Ok(())
}

fn register_shell_path(shim_dir: &Path) -> io::Result<()> {
    let block = profile_path_block(shim_dir);
    for profile in shell_profiles()? {
        let content = fs::read_to_string(&profile).unwrap_or_default();
        if !content.contains(&block) {
            let prefix = if content.ends_with('\n') || content.is_empty() {
                ""
            } else {
                "\n"
            };
            fs::write(profile, format!("{content}{prefix}{block}\n"))?;
        }
    }
    Ok(())
}

fn shell_profiles() -> io::Result<Vec<PathBuf>> {
    let home = home_dir()?;
    Ok(vec![home.join(".zshrc"), home.join(".bash_profile")])
}

fn profile_path_block(shim_dir: &Path) -> String {
    format!(
        "{PROFILE_MARKER}\nexport PATH=\"{}:$PATH\"",
        shim_dir.to_string_lossy()
    )
}

fn remove_profile_path_block(content: &str, shim_dir: &Path) -> String {
    let block = profile_path_block(shim_dir);
    content
        .replace(&format!("{block}\n"), "")
        .replace(&block, "")
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
        .is_some_and(|current| current.starts_with(&root))
    {
        let launcher = stable_launcher_path()?;
        if launcher.exists() && current.as_ref() != Some(&launcher) {
            fs::remove_file(launcher)?;
            report.removed("stable Open with Codex app launcher");
        } else {
            report.skipped("stable Open with Codex app launcher");
        }
        report.skipped(
            "Open with Codex app helper directory because the uninstaller is running from it",
        );
        return Ok(());
    }

    fs::remove_dir_all(root)?;
    report.removed("Open with Codex app helper files");
    Ok(())
}

fn find_forward_target() -> io::Result<Option<PathBuf>> {
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
            .is_some_and(|current_dir| current_dir == &dir)
        {
            continue;
        }

        let target = dir.join("codex");
        if target.is_file() && current.as_ref().is_none_or(|current| current != &target) {
            return Ok(Some(target));
        }
    }

    Ok(None)
}
