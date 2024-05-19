use std::env;

use git2::{Cred, PushOptions, RemoteCallbacks, Repository};
use log::info;

pub struct GitRepository {
    repo: Repository,
}

impl GitRepository {
    pub fn clone_repo(
        repo_url: &str,
        local_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Cloning repository from {} to {}", repo_url, local_path);
        let repo = Repository::clone(repo_url, local_path)?;
        Ok(GitRepository { repo })
    }

    pub fn create_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch, &commit, false)?;
        Ok(())
    }

    pub fn commit_changes(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut index = self.repo.index()?;
        index.add_all(
            [".github/workflows/*"].iter(),
            git2::IndexAddOption::DEFAULT,
            None,
        )?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;
        let signature = self.repo.signature()?;
        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;
        Ok(())
    }

    pub fn push_changes(
        &self,
        branch: &str,
        force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut remote = self.repo.find_remote("origin")?;
        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        };

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            //TODO: This should not be here
            let token = env::var("GITHUB_TOKEN").unwrap_or_else(|_| String::from("default_token"));
            Cred::userpass_plaintext("x-access-token", &token)
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_options))?;
        Ok(())
    }

    pub fn checkout_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let obj = match self.repo.revparse_single(&format!("refs/heads/{}", branch)) {
            Ok(obj) => obj,
            Err(_) => {
                self.create_branch(branch)?;
                self.repo
                    .revparse_single(&format!("refs/heads/{}", branch))?
            }
        };
        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head(&format!("refs/heads/{}", branch))?;
        Ok(())
    }
}
