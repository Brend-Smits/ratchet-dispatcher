use std::env;
use std::process::Command;
use log::info;

pub struct GitRepository;

impl GitRepository {
    pub fn clone_repo(
        repo_url: &str,
        local_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Cloning repository from {} to {}", repo_url, local_path);

        let output = Command::new("git")
            .arg("clone")
            .arg(repo_url)
            .arg(local_path)
            .output()?;

        if !output.status.success() {
            return Err(Box::from(format!(
                "Failed to clone repository: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(GitRepository)
    }

    pub fn create_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .arg("branch")
            .arg(branch)
            .output()?;

        if !output.status.success() {
            return Err(Box::from(format!(
                "Failed to create branch: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    pub fn remove_blank_line_changes(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("git diff -U0 -w --no-color --ignore-blank-lines | git apply --cached --ignore-whitespace --unidiff-zero -")
            .output()?;

        if !output.status.success() {
            return Err(Box::from(format!(
                "Failed to remove blank line changes: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    pub fn stage_changes(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .arg("add")
            .arg(".github/workflows/*")
            .output()?;

        if !output.status.success() {
            return Err(Box::from(format!(
                "Failed to stage changes: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    pub fn commit_changes(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .output()?;

        if !output.status.success() {
            return Err(Box::from(format!(
                "Failed to commit changes: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    pub fn push_changes(
        &self,
        branch: &str,
        force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        };

        let output = Command::new("git")
            .arg("push")
            .arg("origin")
            .arg(&refspec)
            .output()?;

        if !output.status.success() {
            return Err(Box::from(format!(
                "Failed to push changes: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    pub fn checkout_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .arg("checkout")
            .arg(branch)
            .output()?;

        if !output.status.success() {
            return Err(Box::from(format!(
                "Failed to checkout branch: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
            }

        Ok(())
    }
}
