use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

use open_with_codex_app::cli::{parse_command, usage, Command};
use open_with_codex_app::platform;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8, String> {
    let mut raw_args = env::args_os();
    let invocation = raw_args
        .next()
        .unwrap_or_else(|| OsString::from("open-with-codex-app"));
    let args: Vec<OsString> = raw_args.collect();
    let invocation_text = invocation.to_string_lossy();

    match parse_command(&invocation_text, args).map_err(|err| format!("{err}\n\n{}", usage()))? {
        Command::Install => {
            let report = platform::install_integrations().map_err(|err| err.to_string())?;
            for line in report.lines {
                println!("{line}");
            }
            Ok(0)
        }
        Command::Uninstall => {
            let report = platform::uninstall_integrations().map_err(|err| err.to_string())?;
            for line in report.lines {
                println!("{line}");
            }
            Ok(0)
        }
        Command::OpenProject { path } => {
            let outcome = platform::open_project(&path).map_err(|err| err.to_string())?;
            println!("{outcome}");
            Ok(0)
        }
        Command::Forward(args) => platform::forward_to_existing_codex(&args)
            .map(|code| code as u8)
            .map_err(|err| err.to_string()),
        Command::Help => {
            println!("{}", usage());
            Ok(0)
        }
    }
}
