use clap::Parser;
use clap_verbosity_flag::Verbosity;
use git2::{BranchType, Cred, PushOptions, RemoteCallbacks, Repository};
use log::{debug, error, info};
use octocrab::{models::pulls::PullRequest, Octocrab};
use std::fs::{self};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::{env, process};

#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    owner: String,
    #[clap(long)]
    repo: String,
    #[clap(long)]
    branch: String,
    #[clap(flatten)]
    verbose: Verbosity,
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
            .send()
            .await?;
        Ok(pr)
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

    fn push_changes(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut remote = self.repo.find_remote("origin")?;
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            //TODO: Fix hardcoded username
            Cred::userpass_plaintext("brend-smits", &env::var("GITHUB_TOKEN").unwrap())
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_options))?;
        Ok(())
    }

    fn checkout_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let obj = self
            .repo
            .revparse_single(&format!("refs/heads/{}", branch))?;
        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head(&format!("refs/heads/{}", branch))?;
        Ok(())
    }
}

fn set_permissions(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755); // rwx for owner, rx for group and others
    fs::set_permissions(path, permissions).map(|_| {
        debug!("Successfully set permissions for {}", path.display());
    })?;
    Ok(())
}

fn upgrade_workflows(local_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Upgrading workflows in {}", local_path);
    let workflows_path = format!("{}/.github/workflows", local_path);
    if Path::new(&workflows_path).exists() {
        debug!("Found workflows directory at {}", workflows_path);
        for entry in fs::read_dir(&workflows_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                debug!("Setting permissions for workflow: {:?}", local_path);
                set_permissions(Path::new(local_path))?;
                debug!("Upgrading workflow: {}", path.display());
                let mut cmd = Command::new("ratchet");
                cmd.arg("pin").arg(path.to_str().unwrap());
                debug!("Running command: {:?}", cmd);
                let output = cmd
                    .output()
                    .map_err(|e| {
                        error!("Failed to run ratchet: {}, removing directory", e);
                        // Remove the entire directory if ratchet fails
                        fs::remove_dir_all(local_path)
                            .map(|_| {
                                debug!("Removed directory: {}", local_path);
                            })
                            .expect("Failed to clean up tmp repository directory");
                    })
                    .expect("Failed to execute ratchet");

                debug!("Ratchet output: {:?}", output);
                if !output.status.success() {
                    error!(
                        "ratchet upgrade failed for {}: {}",
                        path.display(),
                        String::from_utf8_lossy(&output.stderr)
                    );
                    // Remove the entire directory if ratchet fails
                    fs::remove_dir_all(local_path)
                        .map(|_| {
                            debug!("Removed directory: {}", local_path);
                        })
                        .expect("Failed to clean up tmp repository directory");
                    return Err(Box::from("ratchet upgrade command failed"));
                } else {
                    info!(
                        "Successfully upgraded workflow: {}",
                        path.file_name().unwrap().to_str().unwrap()
                    );
                }
            }
        }
    } else {
        error!("No workflows directory found at {}", workflows_path);
        return Err(Box::from("Workflows directory not found"));
    }
    Ok(())
}

// Load environment variables
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .format_module_path(false)
        .format_target(false)
        .init();
    let token = load_env_vars();
    let repo_url = format!("https://github.com/{}/{}.git", args.owner, args.repo);
    let local_path = format!("temp_clones/{}_{}", args.owner, args.repo);

    let git_repo = GitRepository::clone_repo(&repo_url, &local_path)?;

    // Check if the branch exists and create it if it doesn't
    if git_repo
        .repo
        .revparse_single(&format!("refs/heads/{}", args.branch))
        .is_err()
    {
        git_repo.create_branch(&args.branch)?;
    }

    git_repo.checkout_branch(&args.branch)?;

    //TODO: In any case, we should clean up the temp directory. This should be done in a finally block
    if let Err(e) = upgrade_workflows(&local_path) {
        error!("Failed to upgrade workflows: {}", e);
        process::exit(1);
    }

    if let Err(e) = git_repo.commit_changes("Upgrade workflows with ratchet") {
        error!("Failed to commit changes: {}", e);
        process::exit(1);
    }

    if let Err(e) = git_repo.push_changes(&args.branch) {
        error!("Failed to push changes: {}", e);
        process::exit(1);
    }

    let github_client = GitHubClient::new(args.owner, args.repo, token);
    match github_client.create_pull_request(&args.branch).await {
        Ok(pr) => println!("Created PR: {:?}", pr.html_url),
        Err(e) => {
            error!("Failed to create PR: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}
