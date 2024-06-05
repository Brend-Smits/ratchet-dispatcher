use std::{fs, path::Path, process::Command};

use log::{debug, error, info};

use crate::cleanup_clone_dir;

pub fn upgrade_workflows(local_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Upgrading workflows in {}", local_path);
    let workflows_path = format!("{}/.github/workflows", local_path);
    if !Path::new(&workflows_path).exists() {
        error!("No workflows directory found at {}", workflows_path);
        return Err(Box::from("Workflows directory not found"));
    }

    debug!("Found workflows directory at {}", workflows_path);
    for entry in fs::read_dir(&workflows_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Err(e) = upgrade_single_workflow(&path, local_path) {
                error!("Failed to upgrade workflow: {}", e);
                cleanup_clone_dir(local_path);
                return Err(e);
            }
        }
    }

    Ok(())
}

pub fn upgrade_single_workflow(
    path: &Path,
    local_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Upgrading workflow: {}", path.display());

    let output = run_ratchet_command(path)?;

    debug!("Ratchet output: {:?}", output);
    if !output.status.success() {
        error!(
            "ratchet upgrade failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        cleanup_clone_dir(local_path);
        return Err(Box::from("ratchet upgrade command failed"));
    }

    info!(
        "Successfully upgraded workflow: {:?}",
        path.file_name().unwrap().to_str()
    );
    Ok(())
}

fn run_ratchet_command(path: &Path) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("ratchet");
    cmd.arg("pin").arg(path.to_str().unwrap());
    debug!("Running command: {:?}", cmd);

    let output = cmd.output()?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, File},
        io::Write,
    };

    use assert_cmd::Command;
    use mockall::predicate::*;
    use tempfile::{tempdir, TempDir};

    const UNPINNED_WORKFLOW: &str = include_str!("../resources/ci_unpinned.yml");
    const PINNED_WORKFLOW: &str = include_str!("../resources/ci_pinned.yml");

    // #[test]
    // fn test_upgrade_workflows_success() {
    //     env_logger::init();
    //     let tmp_dir = TempDir::new().unwrap();

    //     let workflow_dir = tmp_dir.path().join(".github/workflows");
    //     fs::create_dir_all(&workflow_dir).unwrap();

    //     let workflow_path = workflow_dir.join("ci.yml");
    //     let mut f = File::create(workflow_path.clone()).unwrap();
    //     f.write_all(UNPINNED_WORKFLOW.as_bytes()).unwrap();

    //     let result = upgrade_workflows(tmp_dir.path().to_str().unwrap());

    //     let upgraded_content = fs::read_to_string(&workflow_path).unwrap();
    //     assert_eq!(upgraded_content, PINNED_WORKFLOW);
    // }

    #[test]
    fn test_upgrade_workflows_missing_directory() {
        let dir = tempdir().unwrap();

        let result = upgrade_workflows(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    // #[test]
    // fn test_upgrade_single_workflow_success() {
    //     env_logger::init();
    //     let dir = tempdir().unwrap();
    //     let workflow_path = dir.path().join("ci.yml");
    //     fs::write(&workflow_path, UNPINNED_WORKFLOW).unwrap();
    //     error!("Temporary directory created at: {}", dir.path().display());
    //     error!("Workflow path: {}", workflow_path.display());
    //     let result = upgrade_single_workflow(&workflow_path, dir.path().to_str().unwrap());
    //     assert!(result.is_ok());

    //     let upgraded_content = fs::read_to_string(&workflow_path).unwrap();
    //     assert_eq!(upgraded_content, PINNED_WORKFLOW);
    // }

    // #[test]
    // fn test_upgrade_single_workflow_failure() {
    //     env_logger::init();
    //     let dir = tempdir().unwrap();
    //     let workflow_path = dir.path().join("ci.yml");
    //     fs::write(&workflow_path, UNPINNED_WORKFLOW).unwrap();

    //     Command::new("touch").arg("fake-ratchet").assert().failure();

    //     let result = upgrade_single_workflow(&workflow_path, dir.path().to_str().unwrap());
    //     assert!(result.is_err());
    // }
}
