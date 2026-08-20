/// Reverse-DNS label used for the LaunchAgent.
pub const LAUNCH_AGENT_LABEL: &str = "com.keyclack.agent";

/// Errors that can occur while installing or uninstalling the login item.
#[derive(Debug)]
pub enum LoginItemError {
    NoHomeDir,
    Io(std::io::Error),
    /// launchctl exited non-zero.
    LaunchctlFailed { code: Option<i32>, stderr: String },
    /// std::env::current_exe() failed.
    ExePathUnknown,
}

impl std::fmt::Display for LoginItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginItemError::NoHomeDir => write!(f, "could not determine home directory"),
            LoginItemError::Io(e) => write!(f, "login item I/O error: {e}"),
            LoginItemError::LaunchctlFailed { code, stderr } => {
                write!(f, "launchctl failed (code {code:?}): {stderr}")
            }
            LoginItemError::ExePathUnknown => write!(f, "could not determine current executable path"),
        }
    }
}

impl std::error::Error for LoginItemError {}

/// ~/Library/LaunchAgents/com.keyclack.agent.plist
pub fn plist_path() -> Result<std::path::PathBuf, LoginItemError> {
    let home = directories::BaseDirs::new().ok_or(LoginItemError::NoHomeDir)?;
    Ok(home
        .home_dir()
        .join("Library/LaunchAgents")
        .join(format!("{LAUNCH_AGENT_LABEL}.plist")))
}

// SPEC-QUESTION: 4.9 specifies this as part of the module's public contract, but no step in
// section 5's function-level plan calls it — the UI treats config.start_on_login as the source
// of truth instead. Kept as specified for API completeness.
/// True if the plist file exists.
#[allow(dead_code)]
pub fn is_installed() -> Result<bool, LoginItemError> {
    Ok(plist_path()?.exists())
}

/// Write the plist pointing at the current executable, then `launchctl load -w <path>`.
pub fn install() -> Result<(), LoginItemError> {
    let exe_path = std::env::current_exe().map_err(|_| LoginItemError::ExePathUnknown)?;
    let path = plist_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(LoginItemError::Io)?;
    }

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LAUNCH_AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        exe_path = exe_path.display()
    );

    std::fs::write(&path, plist).map_err(LoginItemError::Io)?;

    let output = std::process::Command::new("launchctl")
        .arg("load")
        .arg("-w")
        .arg(&path)
        .output()
        .map_err(LoginItemError::Io)?;

    if !output.status.success() {
        return Err(LoginItemError::LaunchctlFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(())
}

/// `launchctl unload -w <path>`, then delete the plist. Missing plist is not an error.
pub fn uninstall() -> Result<(), LoginItemError> {
    let path = plist_path()?;

    if !path.exists() {
        return Ok(());
    }

    let output = std::process::Command::new("launchctl")
        .arg("unload")
        .arg("-w")
        .arg(&path)
        .output()
        .map_err(LoginItemError::Io)?;

    if !output.status.success() {
        return Err(LoginItemError::LaunchctlFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    std::fs::remove_file(&path).map_err(LoginItemError::Io)?;

    Ok(())
}
