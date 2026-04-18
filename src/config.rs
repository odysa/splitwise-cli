use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

fn config_path() -> Result<PathBuf> {
    let home = home::home_dir().context("could not determine home directory")?;
    Ok(home.join(".splitwise-cli"))
}

pub fn save_token(key: &str) -> Result<()> {
    let path = config_path()?;
    fs::write(&path, key.trim())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn load_token() -> Result<String> {
    if let Ok(key) = std::env::var("SPLITWISE_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    let path = config_path()?;
    let key = fs::read_to_string(&path).with_context(|| {
        format!(
            "could not read API key from {}\nRun `splitwise auth <key>` or set SPLITWISE_API_KEY",
            path.display()
        )
    })?;
    let key = key.trim().to_string();
    if key.is_empty() {
        anyhow::bail!("API key is empty. Run `splitwise auth <key>` or set SPLITWISE_API_KEY");
    }
    Ok(key)
}
