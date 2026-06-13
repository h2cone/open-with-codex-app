use std::ffi::OsString;
use std::path::PathBuf;

use open_with_codex_app::cli::{parse_command, Command};
use open_with_codex_app::platform::macos_assets::workflow_shell_script;
use open_with_codex_app::registration::{plan_registration, RegistrationAction, RegistrationState};

#[cfg(windows)]
use open_with_codex_app::platform::windows::{
    cli_shim_path_for_local_app_data, context_menu_command, remove_path_entry_from_value,
    strip_windows_verbatim_prefix,
};

#[test]
fn installer_invocation_without_args_registers_integrations() {
    let command = parse_command("open-with-codex-app", Vec::<OsString>::new()).unwrap();

    assert_eq!(command, Command::Install);
}

#[test]
fn codex_app_invocation_opens_project_path() {
    let command = parse_command(
        "codex",
        vec![OsString::from("app"), OsString::from("/tmp/project")],
    )
    .unwrap();

    assert_eq!(
        command,
        Command::OpenProject {
            path: "/tmp/project".into()
        }
    );
}

#[test]
fn codex_wrapper_forwards_unknown_commands_to_existing_cli() {
    let command = parse_command("codex", vec![OsString::from("--version")]).unwrap();

    assert_eq!(command, Command::Forward(vec![OsString::from("--version")]));
}

#[test]
fn uninstall_invocation_removes_integrations() {
    let command = parse_command("open-with-codex-app", vec![OsString::from("uninstall")]).unwrap();

    assert_eq!(command, Command::Uninstall);
}

#[test]
fn registration_skips_everything_when_codex_app_is_missing() {
    let state = RegistrationState {
        codex_app_installed: false,
        context_menu_registered: false,
        cli_registered: false,
        cli_path_registered: false,
    };

    assert_eq!(
        plan_registration(&state),
        vec![RegistrationAction::SkipMissingCodexApp]
    );
}

#[test]
fn registration_skips_when_everything_is_already_registered() {
    let state = RegistrationState {
        codex_app_installed: true,
        context_menu_registered: true,
        cli_registered: true,
        cli_path_registered: true,
    };

    assert_eq!(plan_registration(&state), Vec::<RegistrationAction>::new());
}

#[test]
fn registration_plans_only_missing_integration_pieces() {
    let state = RegistrationState {
        codex_app_installed: true,
        context_menu_registered: false,
        cli_registered: true,
        cli_path_registered: false,
    };

    assert_eq!(
        plan_registration(&state),
        vec![
            RegistrationAction::RegisterContextMenu,
            RegistrationAction::RegisterCliPath
        ]
    );
}

#[cfg(windows)]
#[test]
fn windows_context_menu_command_routes_folder_to_app_subcommand() {
    let command = context_menu_command(r"C:\Tools\open-with-codex-app.exe".as_ref(), "%1");

    assert_eq!(command, r#""C:\Tools\open-with-codex-app.exe" app "%1""#);
}

#[cfg(windows)]
#[test]
fn windows_cli_shim_path_does_not_collide_with_official_codex_bin() {
    let path = cli_shim_path_for_local_app_data(r"C:\Users\me\AppData\Local".as_ref());

    assert_eq!(
        path,
        PathBuf::from(r"C:\Users\me\AppData\Local\OpenAI\Codex\OpenWithCodexApp\bin\codex.exe")
    );
}

#[cfg(windows)]
#[test]
fn windows_project_path_strips_verbatim_drive_prefix_before_launch() {
    let path = strip_windows_verbatim_prefix(r"\\?\C:\Users\me\project".as_ref());

    assert_eq!(path, PathBuf::from(r"C:\Users\me\project"));
}

#[cfg(windows)]
#[test]
fn windows_project_path_strips_verbatim_unc_prefix_before_launch() {
    let path = strip_windows_verbatim_prefix(r"\\?\UNC\server\share\project".as_ref());

    assert_eq!(path, PathBuf::from(r"\\server\share\project"));
}

#[cfg(windows)]
#[test]
fn windows_path_cleanup_removes_only_registered_shim_directory() {
    let shim_dir = r"C:\Users\me\AppData\Local\OpenAI\Codex\OpenWithCodexApp\bin".as_ref();
    let old_path =
        r"C:\Tools;C:\Users\me\AppData\Local\OpenAI\Codex\OpenWithCodexApp\bin;C:\Windows";

    let new_path = remove_path_entry_from_value(old_path, shim_dir);

    assert_eq!(new_path, r"C:\Tools;C:\Windows");
}

#[test]
fn macos_workflow_script_opens_each_folder_argument() {
    let script = workflow_shell_script(
        "/Users/me/Library/Application Support/OpenAI/Codex/OpenWithCodexApp/open-with-codex-app",
    );

    assert!(script.contains("for item in \"$@\"; do"));
    assert!(script.contains("\"/Users/me/Library/Application Support/OpenAI/Codex/OpenWithCodexApp/open-with-codex-app\" app \"$item\""));
}
