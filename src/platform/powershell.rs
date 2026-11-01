use std::fs;
use std::io;
use std::path::Path;

const PROFILE_BLOCK: &str = "\r\n# >>> Open with Codex app >>>\r\nif (Test-Path -LiteralPath \"$env:LOCALAPPDATA\\OpenAI\\Codex\\OpenWithCodexApp\\powershell.ps1\") {\r\n    . \"$env:LOCALAPPDATA\\OpenAI\\Codex\\OpenWithCodexApp\\powershell.ps1\"\r\n}\r\n# <<< Open with Codex app <<<\r\n";

pub const ROUTER_SCRIPT: &str = r#"# Managed by Open with Codex app. Only the app subcommand is intercepted.
function global:codex {
    $desktopShim = Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\OpenWithCodexApp\bin\codex.exe'
    if ($args.Count -gt 0 -and $args[0] -ceq 'app') {
        & $desktopShim @args
        return
    }

    $officialCli = Get-Command -Name codex.exe,codex.cmd,codex.bat -All -CommandType Application -ErrorAction SilentlyContinue |
        Where-Object { $_.Source -ine $desktopShim } |
        Select-Object -First 1
    if ($null -eq $officialCli) {
        Write-Error 'No existing Codex CLI was found.'
        $global:LASTEXITCODE = 1
        return
    }
    & $officialCli.Source @args
}
"#;

// Obtain the shell object hosted by the desktop's Explorer process, rather than
// inheriting the calling terminal's elevated token. User input is passed only
// through environment values, never interpolated into this PowerShell code.
pub const EXPLORER_LAUNCH_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding
try {
    $shell = New-Object -ComObject Shell.Application
    $desktopHandle = 0
    $desktop = $shell.Windows().FindWindowSW(0, 0, 8, [ref]$desktopHandle, 1)
    if ($null -eq $desktop) { throw 'The Explorer desktop shell is unavailable.' }
    $desktop.Document.Application.ShellExecute(
        $env:OPEN_WITH_CODEX_TARGET,
        $env:OPEN_WITH_CODEX_ARGUMENTS,
        '', 'open', 1)
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
"#;

pub fn write_router(path: &Path) -> io::Result<bool> {
    let old = read_or_empty(path)?;
    if old == ROUTER_SCRIPT.as_bytes() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, ROUTER_SCRIPT)?;
    Ok(true)
}

pub fn install_profile(path: &Path) -> io::Result<bool> {
    let mut contents = read_or_empty(path)?;
    let block = encoded_profile_block(&contents);
    if contents.windows(block.len()).any(|bytes| bytes == block) {
        return Ok(false);
    }
    contents.extend_from_slice(&block);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(true)
}

pub fn uninstall_profile(path: &Path) -> io::Result<bool> {
    let mut contents = read_or_empty(path)?;
    let block = encoded_profile_block(&contents);
    let Some(start) = contents
        .windows(block.len())
        .position(|bytes| bytes == block)
    else {
        return Ok(false);
    };
    contents.drain(start..start + block.len());
    fs::write(path, contents)?;
    Ok(true)
}

fn read_or_empty(path: &Path) -> io::Result<Vec<u8>> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

fn encoded_profile_block(contents: &[u8]) -> Vec<u8> {
    if contents.starts_with(&[0xff, 0xfe]) {
        PROFILE_BLOCK
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect()
    } else if contents.starts_with(&[0xfe, 0xff]) {
        PROFILE_BLOCK
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect()
    } else {
        PROFILE_BLOCK.as_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_install_is_idempotent_and_uninstall_preserves_user_bytes() {
        for original in [
            b"# user profile\nWrite-Host 'hello'".to_vec(),
            [vec![0xef, 0xbb, 0xbf], "# 用户配置\r\n".as_bytes().to_vec()].concat(),
            [
                vec![0xff, 0xfe],
                "# 用户配置\r\n"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect(),
            ]
            .concat(),
            [
                vec![0xfe, 0xff],
                "# user profile\r\n"
                    .encode_utf16()
                    .flat_map(u16::to_be_bytes)
                    .collect(),
            ]
            .concat(),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("profile.ps1");
            fs::write(&path, &original).unwrap();

            assert!(install_profile(&path).unwrap());
            assert!(!install_profile(&path).unwrap());
            assert!(fs::read(&path).unwrap().starts_with(&original));
            assert!(uninstall_profile(&path).unwrap());
            assert_eq!(fs::read(&path).unwrap(), original);
            assert!(!uninstall_profile(&path).unwrap());
        }
    }

    #[test]
    fn missing_profiles_can_be_created_and_missing_uninstall_is_a_noop() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PowerShell/profile.ps1");
        assert!(!uninstall_profile(&path).unwrap());
        assert!(install_profile(&path).unwrap());
        assert!(uninstall_profile(&path).unwrap());
        assert!(fs::read(path).unwrap().is_empty());
    }
}
