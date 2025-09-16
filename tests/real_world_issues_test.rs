use ratchet_dispatcher::git::GitRepository;
use tempfile::tempdir;

fn create_problematic_workflow(repo_path: &str) {
    let content = r#"name: Issue Ops Onboard User Request
on:
  workflow_dispatch:
    inputs:
      issue_number:
        description: 'Issue number'
        required: true
        type: string

jobs:
  parse-incoming-onboard-request:
    runs-on: ubuntu-latest
    outputs:
      ghname: ${{ steps.prepare.outputs.ghname }}
      email: ${{ steps.prepare.outputs.email }}
    steps:
        - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 #v5.0.0
        - name: Fetch the issue
          id: read_issue_body
          run:
            echo "body=$(gh issue view ${{ inputs.issue_number }} --repo ${{
            github.repo }} --json body --jq '.body')" >> $GITHUB_OUTPUT

        - name: Parse issue form body 
          id: parse
          uses: zentered/issue-forms-body-parser@v2.2.0
          with:
            body: ${{ steps.read_issue_body.output.body }}

  check-user-exists:
    needs: parse-incoming-onboard-request
    runs-on: ubuntu-latest
    steps:
      - name: Call GitHub API to check if user exists
        id: call_github_api
        uses: actions/github-script@v7
        with:
          github-token: ${{ secrets.ONBOARDER_GITHUB_TOKEN }}
          script: |
            try {
              const response = await github.rest.orgs.getMembershipForUser({
                org: 'example-rg',
                username: '${{needs.parse-incoming-onboard-request.outputs.ghname}}'
              });
              return response.status;
            } catch (error) {
              return null;
            }

      - name: Find Comment
        uses: peter-evans/find-comment@v3
        id: fc
        if: ${{ steps.condition.outputs.user_exists == 'false' }}
        with:
          issue-number: ${{ github.event.issue.number }}
          comment-author: 'github-actions[bot]'
          body-includes: I'm sorry, but I couldn't find the user

  execute-onboard-request:
    needs: [parse-incoming-onboard-request, check-user-exists]
    runs-on: ubuntu-latest
    container:
      image: project-docker-rnd.artifactory-ehv.ta.company.com/project-member-onboarder:latest
      credentials:
        username: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_USER }}
        password: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_PASS }}
      env:
        ARTIFACTORY_USER: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_USER }}
        ARTIFACTORY_PASS: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_PASS }}
    steps:
        - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 #v5.0.0

        - name: Get Github Token
          uses: actions/create-github-app-token@a8d616148505b5069dccd32f177bb87d7f39123b # v2.1.1
          id: token
          with:
            app-id: 349757
            private-key: ${{ secrets.ONBOARDER_project_MANAGER_GITHUB_APP_PRIVATE_KEY }}
            owner: ${{ github.repository_owner }}

        - name: Download Rivendell Release Artifact
          uses: dsaltares/fetch-gh-release-asset@aa2ab1243d6e0d5b405b973c89fa4d06a2d0fff7
          with:
            file: 'rivendell'
            target: 'rivendell'
            token: ${{steps.token.outputs.token}}

        - name: Create Pull Request
          id: create_pull_request
          uses: peter-evans/create-pull-request@v7
          with:
            token: ${{ secrets.ONBOARDER_GITHUB_TOKEN }}
            commit-message: "chore(onboard): Onboard user."
            title: "chore(onboard): Onboard user"
            branch: "automated-onboard-user"
"#;

    std::fs::create_dir_all(format!("{}/.github/workflows", repo_path))
        .expect("Failed to create workflows dir");
    std::fs::write(
        format!("{}/.github/workflows/problematic-workflow.yml", repo_path),
        content,
    )
    .expect("Failed to write workflow file");
}

#[test]
fn test_real_world_yaml_corruption_issues() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path().to_string_lossy();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path.to_string())
        .output()
        .expect("Failed to init git repo");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path.to_string())
        .output()
        .expect("Failed to set git user email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path.to_string())
        .output()
        .expect("Failed to set git user name");

    // Create problematic workflow and commit
    create_problematic_workflow(&repo_path);

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path.to_string())
        .output()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path.to_string())
        .output()
        .expect("Failed to commit");

    // Simulate running ratchet pin command that would modify the workflow with corruption
    let corrupted_content = r#"name: Issue Ops Onboard User Request
on:
  workflow_dispatch:
    inputs:
      issue_number:
        description: 'Issue number'
        required: true
        type: string

jobs:
  parse-incoming-onboard-request:
    runs-on: ubuntu-latest
    outputs:
      ghname: ${{ steps.prepare.outputs.ghname }}
      email: ${{ steps.prepare.outputs.email }}
    steps:
      - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 #v5.0.0
      - name: Fetch the issue
        id: read_issue_body
        run:
          echo "body=$(gh issue view ${{ inputs.issue_number }} --repo ${{
          github.repo }} --json body --jq '.body')" >> $GITHUB_OUTPUT

        uses: zentered/issue-forms-body-parser@93bd9fdcb3679be1889d2006e9c2cf496899402e # ratchet:zentered/issue-forms-body-parser@v2.2.0
        id: parse
        body: ${{ steps.read_issue_body.output.body }}
        with:
          body: ${{ steps.read_issue_body.output.body }}

  check-user-exists:
    needs: parse-incoming-onboard-request
    runs-on: ubuntu-latest
    steps:
      - name: Call GitHub API to check if user exists
        uses: actions/github-script@f28e40c7f34bde8b3046d885e986cb6290c5673b # ratchet:actions/github-script@v7
        with:
        with:
          github-token: ${{ secrets.ONBOARDER_GITHUB_TOKEN }}
          script: |
            try {
              const response = await github.rest.orgs.getMembershipForUser({
                org: 'example-rg',
                username: '${{needs.parse-incoming-onboard-request.outputs.ghname}}'
              });
              return response.status;
            } catch (error) {
              return null;
            }

        uses: peter-evans/find-comment@3eae4d37986fb5a8592848f6a574fdf654e61f9e # ratchet:peter-evans/find-comment@v3
        id: fc
        id: fc
        if: ${{ steps.condition.outputs.user_exists == 'false' }}
        with:
          issue-number: ${{ github.event.issue.number }}
          comment-author: 'github-actions[bot]'
          body-includes: I'm sorry, but I couldn't find the user

  execute-onboard-request:
    needs: [parse-incoming-onboard-request, check-user-exists]
    runs-on: ubuntu-latest
    container:
      image: project-docker-rnd.artifactory-ehv.ta.company.com/project-member-onboarder:latest
      credentials:
        username: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_USER }}
        password: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_PASS }}
      env:
        ARTIFACTORY_USER: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_USER }}
        ARTIFACTORY_PASS: ${{ secrets.DISTRIBUTED_SECRET_ARTIFACTORY_PASS }}
      - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 #v5.0.0
      - name: Get Github Token

        uses: actions/create-github-app-token@a8d616148505b5069dccd32f177bb87d7f39123b # v2.1.1
        id: token
        id: token
        with:
          app-id: 349757
          private-key: ${{ secrets.ONBOARDER_project_MANAGER_GITHUB_APP_PRIVATE_KEY }}
          owner: ${{ github.repository_owner }}

        uses: dsaltares/fetch-gh-release-asset@aa2ab1243d6e0d5b405b973c89fa4d06a2d0fff7
        with:
        with:
          file: 'rivendell'
          target: 'rivendell'
          token: ${{steps.token.outputs.token}}

        uses: peter-evans/create-pull-request@271a8d0340265f705b14b6d32b9829c1cb33d45e # ratchet:peter-evans/create-pull-request@v7
        id: create_pull_request
        token: ${{ secrets.ONBOARDER_GITHUB_TOKEN }} # PAT is needed to trigger workflows
        with:
          token: ${{ secrets.ONBOARDER_GITHUB_TOKEN }}
          commit-message: "chore(onboard): Onboard user."
          title: "chore(onboard): Onboard user"
          branch: "automated-onboard-user"
"#;

    // Write the corrupted content to simulate what ratchet dispatcher is currently producing
    std::fs::write(
        format!("{}/.github/workflows/problematic-workflow.yml", repo_path),
        corrupted_content,
    )
    .expect("Failed to write corrupted workflow file");

    // Test our surgical staging
    let git_repo = GitRepository::open(repo_path.to_string()).expect("Failed to open repo");
    git_repo
        .stage_changes(false)
        .expect("Failed to stage changes");

    // Check what was staged - should be only uses: changes
    let staged_diff = std::process::Command::new("git")
        .args(["diff", "--cached"])
        .current_dir(repo_path.to_string())
        .output()
        .expect("Failed to get staged diff");

    let staged_content = String::from_utf8_lossy(&staged_diff.stdout);
    println!("Staged content:\n{}", staged_content);

    // Verify the structural issues are detected
    // This test should reveal that our current implementation stages corrupted YAML
    let added_lines: Vec<&str> = staged_content
        .lines()
        .filter(|line| line.starts_with("+") && !line.starts_with("+++"))
        .collect();
    let removed_lines: Vec<&str> = staged_content
        .lines()
        .filter(|line| line.starts_with("-") && !line.starts_with("---"))
        .collect();

    println!("Added lines: {:?}", added_lines);
    println!("Removed lines: {:?}", removed_lines);

    // These should fail with current implementation, highlighting the problems:

    // Problem 1: Should only have uses: changes, not structural corruption
    // Allow for trailing newline differences which are valid file content changes
    let non_uses_added: Vec<_> = added_lines
        .iter()
        .filter(|line| !line.contains("uses:") && !line.trim().is_empty())
        .collect();
    let non_uses_removed: Vec<_> = removed_lines
        .iter()
        .filter(|line| !line.contains("uses:") && !line.trim().is_empty())
        .collect();

    // Check if non-uses changes are just identical content (newline differences)
    let mut substantial_changes = false;
    if non_uses_added.len() == non_uses_removed.len() {
        for (added, removed) in non_uses_added.iter().zip(non_uses_removed.iter()) {
            let added_content = added.trim_start_matches('+').trim();
            let removed_content = removed.trim_start_matches('-').trim();
            if added_content != removed_content {
                substantial_changes = true;
                break;
            }
        }
    } else {
        substantial_changes = !non_uses_added.is_empty() || !non_uses_removed.is_empty();
    }

    // Only fail if there are substantial non-uses changes (not just whitespace/newlines)
    if substantial_changes {
        panic!("YAML Structure Corruption Detected! Non-uses lines were staged:\n  Added non-uses: {:?}\n  Removed non-uses: {:?}", 
               non_uses_added, non_uses_removed);
    }

    // Problem 2: Should not have duplicate properties
    let added_content = added_lines.join("\n");
    if added_content.matches("id: fc").count() > 1
        || added_content.matches("id: token").count() > 1
        || added_content.matches("with:").count()
            > added_lines
                .iter()
                .filter(|line| line.contains("uses:"))
                .count()
    {
        panic!("Duplicate properties detected in staged content!");
    }

    // Problem 3: Should not have orphaned properties
    let lines_with_orphaned_props = added_lines
        .iter()
        .filter(|line| {
            (line.contains("id:") || line.contains("with:") || line.contains("body:"))
                && !line.contains("uses:")
        })
        .count();

    if lines_with_orphaned_props > 0 {
        panic!("Orphaned properties without proper step structure detected!");
    }
}
