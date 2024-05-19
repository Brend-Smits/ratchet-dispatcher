use clap::Parser;
use clap_verbosity_flag::Verbosity;
use git2::{Cred, PushOptions, RemoteCallbacks, Repository};
use log::{debug, error, info};
use octocrab::{models::pulls::PullRequest, Octocrab};
use std::fs::{self};
use std::path::Path;
use std::process::Command;
use std::{env, process};

#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    repos: String,
    #[clap(long)]
    branch: String,
    #[clap(flatten)]
    verbose: Verbosity,
    #[clap(long)]
    github_username: Option<String>,
}

struct GitHubClient {
    octocrab: Octocrab,
    owner: String,
    repo: String,
}

impl GitHubClient {
    fn new(owner: String, repo: String, token: String) -> Self {
        let octocrab = Octocrab::builder().personal_token(token).build().unwrap();
        GitHubClient {
            octocrab,
            owner,
            repo,
        }
    }

    async fn create_pull_request(
        &self,
        branch: &str,
    ) -> Result<PullRequest, Box<dyn std::error::Error>> {
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .create("Ratchet Upgrades", branch, "main")
            .body("This PR upgrades the workflows using ratchet.")
            .maintainer_can_modify(true)
            .send()
            .await?;
        Ok(pr)
    }

    async fn find_existing_pr(
        &self,
        branch: &str,
    ) -> Result<Option<PullRequest>, Box<dyn std::error::Error>> {
        let pulls = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .head(branch)
            .state(octocrab::params::State::Open)
            .send()
            .await?;

        Ok(pulls.items.into_iter().next())
    }
}

struct GitRepository {
    repo: Repository,
}

impl GitRepository {
    fn clone_repo(repo_url: &str, local_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Cloning repository from {} to {}", repo_url, local_path);
        let repo = Repository::clone(repo_url, local_path)?;
        Ok(GitRepository { repo })
    }

    fn create_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch, &commit, false)?;
        Ok(())
    }

    fn commit_changes(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    fn push_changes(
        &self,
        branch: &str,
        force: bool,
        github_username: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut remote = self.repo.find_remote("origin")?;
        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        };

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            let username = github_username.as_deref().unwrap_or("brend-smits");
            let token = env::var("GITHUB_TOKEN").unwrap_or_else(|_| String::from("default_token"));
            Cred::userpass_plaintext(username, &token)
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_options))?;
        Ok(())
    }

    fn checkout_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
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

fn upgrade_workflows(local_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
                cleanup(local_path);
                return Err(e);
            }
        }
    }

    Ok(())
}

fn upgrade_single_workflow(
    path: &Path,
    local_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Upgrading workflow: {}", path.display());

    let mut cmd = Command::new("ratchet");
    cmd.arg("pin").arg(path.to_str().unwrap());
    debug!("Running command: {:?}", cmd);

    let output = cmd.output().map_err(|e| {
        error!("Failed to run ratchet: {}", e);
        cleanup(local_path);
        e
    })?;

    debug!("Ratchet output: {:?}", output);
    if !output.status.success() {
        error!(
            "ratchet upgrade failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        cleanup(local_path);
        return Err(Box::from("ratchet upgrade command failed"));
    }

    info!(
        "Successfully upgraded workflow: {:?}",
        path.file_name().unwrap().to_str()
    );
    Ok(())
}

fn load_env_vars() -> String {
    dotenv::dotenv().ok();
    match env::var("GITHUB_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            eprintln!("GITHUB_TOKEN environment variable is not set");
            process::exit(1);
        }
    }
}

fn cleanup(local_path: &str) {
    if fs::remove_dir_all(local_path).is_ok() {
        debug!("Cleaned up temporary directory: {}", local_path);
    } else {
        error!("Failed to clean up temporary directory: {}", local_path);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .format_module_path(false)
        .format_target(false)
        .init();
    let token = load_env_vars();
    let repos: Vec<&str> = args.repos.split(',').collect();

    for repo in repos {
        let repo_parts: Vec<&str> = repo.split('/').collect();
        if repo_parts.len() != 2 {
            error!("Invalid repository format: {}", repo);
            continue;
        }
        let owner = repo_parts[0];
        let repo_name = repo_parts[1];
        let repo_url = format!("https://github.com/{}/{}.git", owner, repo_name);
        let local_path = format!("temp_clones/{}_{}", owner, repo_name);

        let git_repo = match GitRepository::clone_repo(&repo_url, &local_path) {
            Ok(repo) => repo,
            Err(e) => {
                error!("Failed to clone repository: {}", e);
                continue;
            }
        };

        if git_repo.checkout_branch(&args.branch).is_err() {
            if let Err(e) = git_repo.create_branch(&args.branch) {
                error!("Failed to create branch: {}", e);
                cleanup(&local_path);
                continue;
            }
        }

        if let Err(e) = upgrade_workflows(&local_path) {
            error!("Failed to upgrade workflows: {}", e);
            cleanup(&local_path);
            continue;
        }

        if let Err(e) = git_repo.commit_changes("Upgrade workflows with ratchet") {
            error!("Failed to commit changes: {}", e);
            cleanup(&local_path);
            continue;
        }

        let github_client =
            GitHubClient::new(owner.to_string(), repo_name.to_string(), token.clone());
        let force_push = match github_client.find_existing_pr(&args.branch).await {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => {
                error!("Failed to check existing PR: {}", e);
                cleanup(&local_path);
                continue;
            }
        };

        if let Err(e) =
            git_repo.push_changes(&args.branch, force_push, args.github_username.clone())
        {
            error!("Failed to push changes: {}", e);
            cleanup(&local_path);
            continue;
        }

        if !force_push {
            match github_client.create_pull_request(&args.branch).await {
                Ok(pr) => info!(
                    "Created PR for {}: {:?}",
                    repo,
                    format!(
                        "{}://{}/{}",
                        pr.html_url.clone().unwrap().scheme().to_string(),
                        pr.html_url.clone().unwrap().domain().unwrap().to_string(),
                        pr.html_url.unwrap().path().to_string()
                    )
                ),
                Err(e) => {
                    error!("Failed to create PR: {}", e);
                    cleanup(&local_path);
                    continue;
                }
            }
        } else {
            info!("Updated existing PR with new changes for {}", repo);
        }

        cleanup(&local_path);
    }

    Ok(())
}
