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

pub async fn upgrade_workflows(local_path: &str, clean_comment: bool) -> Result<()> {
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
            if let Err(e) = upgrade_single_workflow(&path, clean_comment) {
                error!("Failed to upgrade workflow {}: {}", path.display(), e);
                // Continue with other files instead of failing completely
            }
        }
    }

    Ok(())
}

pub fn upgrade_single_workflow(path: &Path, clean_comment: bool) -> Result<()> {
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
        let mut content_after = std::fs::read_to_string(path).with_context(|| {
            format!(
                "Failed to read workflow file after upgrade: {}",
                path.display()
            )
        })?;
        debug!(
            "Content after upgrade (first 200 chars): {}",
            &content_after.chars().take(200).collect::<String>()
        );

        // Clean ratchet comments if requested
        if clean_comment {
            debug!("Cleaning ratchet comments in {}", path.display());
            content_after = clean_ratchet_comments(&content_after);
            std::fs::write(path, &content_after).with_context(|| {
                format!("Failed to write cleaned content to {}", path.display())
            })?;
            debug!("Successfully cleaned ratchet comments");
        }
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

pub fn clean_ratchet_comments(content: &str) -> String {
    let mut cleaned_lines = Vec::new();

    for line in content.lines() {
        if let Some(comment_start) = line.find("# ratchet:") {
            // Extract the part before the comment
            let before_comment = &line[..comment_start];

            // Extract the part after "# ratchet:"
            let comment_part = &line[comment_start + "# ratchet:".len()..];

            // Look for the @ symbol and extract the version after it
            if let Some(at_pos) = comment_part.find('@') {
                let version_part = &comment_part[at_pos + 1..].trim();

                // Clean the version part - remove any trailing whitespace or comments
                let cleaned_version = version_part.split_whitespace().next().unwrap_or("");

                if !cleaned_version.is_empty() {
                    // Reconstruct the line with just the version
                    let version_prefix = if cleaned_version.starts_with('v') {
                        ""
                    } else {
                        "v"
                    };
                    let cleaned_line =
                        format!("{}# {}{}", before_comment, version_prefix, cleaned_version);
                    cleaned_lines.push(cleaned_line);
                } else {
                    // If we can't extract a version, keep the original line
                    cleaned_lines.push(line.to_string());
                }
            } else {
                // No @ symbol found, keep the original line
                cleaned_lines.push(line.to_string());
            }
        } else {
            // Line doesn't contain ratchet comment, keep as-is
            cleaned_lines.push(line.to_string());
        }
    }

    let mut result = cleaned_lines.join("\n");

    // Preserve the original trailing newline if it existed
    if content.ends_with('\n') {
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upgrade_workflows_missing_directory() {
        let dir = tempfile::tempdir().expect("Failed to create temp directory");

        let result = upgrade_workflows(
            dir.path().to_str().expect("Invalid temp directory path"),
            false, // clean_comment = false for test
        )
        .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_clean_ratchet_comments() {
        let input = "# ratchet:actions/checkout@v4";
        let expected = "# v4";
        assert_eq!(clean_ratchet_comments(input), expected);

        let input_with_dot = "# ratchet:actions/github-script@v7.0.1";
        let expected_with_dot = "# v7.0.1";
        assert_eq!(clean_ratchet_comments(input_with_dot), expected_with_dot);

        let regular_comment = "# This is a regular comment";
        assert_eq!(clean_ratchet_comments(regular_comment), regular_comment);

        let multiple_lines = "name: CI\n# ratchet:actions/checkout@v4\nruns-on: ubuntu-latest\n# ratchet:actions/setup-node@v3";
        let expected_multiple = "name: CI\n# v4\nruns-on: ubuntu-latest\n# v3";
        assert_eq!(clean_ratchet_comments(multiple_lines), expected_multiple);

        // Test newline preservation
        let input_with_newline = "# ratchet:actions/checkout@v4\n";
        let expected_with_newline = "# v4\n";
        assert_eq!(
            clean_ratchet_comments(input_with_newline),
            expected_with_newline
        );

        let input_without_newline = "# ratchet:actions/checkout@v4";
        let expected_without_newline = "# v4";
        assert_eq!(
            clean_ratchet_comments(input_without_newline),
            expected_without_newline
        );
    }
}
