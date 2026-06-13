use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Install,
    Uninstall,
    OpenProject { path: PathBuf },
    Forward(Vec<OsString>),
    Help,
}

pub fn parse_command(invocation: &str, args: Vec<OsString>) -> Result<Command, String> {
    let invoked_as_codex = is_codex_invocation(invocation);

    if args.is_empty() {
        return if invoked_as_codex {
            Ok(Command::Forward(args))
        } else {
            Ok(Command::Install)
        };
    }

    let first = args[0].to_string_lossy();
    match first.as_ref() {
        "app" | "open" => parse_open_project(args),
        "install" | "register" => Ok(Command::Install),
        "uninstall" | "remove" | "unregister" => Ok(Command::Uninstall),
        "--help" | "-h" | "help" => Ok(Command::Help),
        _ if invoked_as_codex => Ok(Command::Forward(args)),
        _ => Err(format!("unknown command: {first}")),
    }
}

pub fn usage() -> &'static str {
    "Usage:\n  open-with-codex-app                    Register integrations\n  open-with-codex-app uninstall          Remove integrations\n  open-with-codex-app app <dir>          Open a directory in Codex app\n  codex app <dir>                        Open a directory in Codex app"
}

fn parse_open_project(args: Vec<OsString>) -> Result<Command, String> {
    if args.len() != 2 {
        return Err("expected exactly one directory after 'app'".to_string());
    }

    Ok(Command::OpenProject {
        path: PathBuf::from(&args[1]),
    })
}

fn is_codex_invocation(invocation: &str) -> bool {
    Path::new(invocation)
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("codex"))
}
