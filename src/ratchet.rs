use anyhow::{anyhow, Context, Result};
use std::{fs, path::Path, process::Command};

use log::{debug, error, info};

pub fn check_ratchet_availability() -> Result<()> {
    debug!("Checking if ratchet is available...");

    let output = Command::new("ratchet").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            info!("Ratchet is available: {}", version.trim());
            Ok(())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!(
                "Ratchet command failed: {} (exit code: {:?})", 
                stderr.trim(),
                output.status.code()
            ))
        }
        Err(e) => {
            Err(anyhow!(
                "Ratchet is not installed or not available in PATH. Please install ratchet first. Error: {}
                
You can install ratchet using:
- go install github.com/sethvargo/ratchet@latest
- Binary: Download from https://github.com/sethvargo/ratchet/releases
- Or ensure it's in your CI environment's PATH", 
                e
            ))
        }
    }
}

pub async fn upgrade_workflows(local_path: &str) -> Result<()> {
    info!("Upgrading workflows in {}", local_path);

    // Check if ratchet is installed and available
    check_ratchet_availability()?;

    let workflows_path = format!("{}/.github/workflows", local_path);
    if !Path::new(&workflows_path).exists() {
        error!("No workflows directory found at {}", workflows_path);
        return Err(anyhow!("Workflows directory not found"));
    }

    debug!("Found workflows directory at {}", workflows_path);
    for entry in fs::read_dir(&workflows_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            // Instead of returning an error, we continue
            let _ = upgrade_single_workflow(&path);
        }
    }

    Ok(())
}

pub fn upgrade_single_workflow(path: &Path) -> Result<()> {
    debug!("Upgrading workflow: {}", path.display());

    // Check if file exists and get its content before upgrade
    if path.exists() {
        let content_before = std::fs::read_to_string(path).with_context(|| {
            format!(
                "Failed to read workflow file before upgrade: {}",
                path.display()
            )
        })?;
        debug!(
            "Content before upgrade (first 200 chars): {}",
            &content_before.chars().take(200).collect::<String>()
        );
    }

    let output = run_ratchet_command(path)?;

    debug!("Ratchet output: {:?}", output);
    if !output.status.success() {
        error!(
            "ratchet upgrade failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(anyhow!(
            "ratchet upgrade command for path {} failed",
            path.display()
        ));
    }

    // Check content after upgrade
    if path.exists() {
        let content_after = std::fs::read_to_string(path).with_context(|| {
            format!(
                "Failed to read workflow file after upgrade: {}",
                path.display()
            )
        })?;
        debug!(
            "Content after upgrade (first 200 chars): {}",
            &content_after.chars().take(200).collect::<String>()
        );
    }

    info!(
        "Successfully upgraded workflow: {}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
    );

    Ok(())
}

fn run_ratchet_command(path: &Path) -> Result<std::process::Output> {
    let mut cmd = Command::new("ratchet");
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid path: {:?}", path))?;
    cmd.arg("pin").arg(path_str);
    debug!("Running command: {:?}", cmd);

    let output = cmd.output()?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upgrade_workflows_missing_directory() {
        let dir = tempfile::tempdir().expect("Failed to create temp directory");

        let result =
            upgrade_workflows(dir.path().to_str().expect("Invalid temp directory path")).await;
        assert!(result.is_err());
    }
}
