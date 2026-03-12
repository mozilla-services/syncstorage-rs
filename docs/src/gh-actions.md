# GitHub Actions

We lint, test, build, and deploy Syncserver-rs using GitHub Actions. We have a number of conventions to follow for security and maintainability purposes and this documentation lays this out.

For general information on GitHub Actions, please see the [GitHub Actions official documentation](https://docs.github.com/en/actions).

## Guidelines for Maintaining GitHub Actions

### Code Review & Approval

- Require code reviews for all workflow changes; enforce this via branch protection rules and `CODEOWNERS`
- When introducing any new third-party actions, request review from the GitHub Enterprise (GHE) team and Security team. Go to *Github Actions and Applications Security Review Changes* in our internal mana space to submit or speak to a member of the security team. When organization-level requests are made, the GHE team routes them to the Security team for review and approval before granting access.

The following permission requests are **automatically approved** by the GHE team without a security review:

- Read-only permissions for all publicly available resources (code, pull requests, issues, etc.) across all public repositories in any Mozilla organization
- Permission removal or decommissioning requests of any kind

The following require **security review and approval** before access is granted:

- Read-only permissions for non-public resources (members, teams, settings, etc.) in public repositories
- Read-only permissions for private or internal repositories
- Write permissions for any public, private, or internal repository

A list of pre-approved apps and actions is maintained in the (GHE Pre-Approved List)[https://github.com/MoCo-GHE-Admin/Approved-GHE-add-ons/blob/main/GitHub_Applications.md].

### Action Pinning & Updates

- Pin all actions to a commit hash instead of a version tag — this applies to Mozilla, GitHub, and especially third-party actions
- Ensure GitHub Actions are kept up to date using [Dependabot](https://docs.github.com/en/code-security/dependabot/working-with-dependabot/keeping-your-actions-up-to-date-with-dependabot)
- Configure a cooldown period of 7 days for Dependabot updates across all ecosystems.

### Permissions & Least Privilege

- Use least privilege for the GitHub token configured in each workflow.
- Avoid 'write' or 'admin' permissions unless absolutely necessary.
- If no specific permissions are required, set `permissions: {}` at the job level.
- Explicitly set `persist-credentials: false` when using the `actions/checkout` action.
- Disable any unnecessary jobs.

### Injection & Script Safety

- Review all scripts run in workflows for code injection risk, including both inline and external scripts.
- Pass all parameters to workflows using environment variables — do not use GitHub Actions expressions (`${{ }}`) for this; applies to `github.event.*`, `github.ref_name`, input, and output parameters
- Do not use GitHub Actions expressions for env variables — use `$VARIABLE` instead of `${{ env.VARIABLE }}`

### Event Trigger Safety

- Avoid using `pull_request_target` and `workflow_run` event triggers whenever possible
- If these triggers are necessary, target only trusted branches and do not check out untrusted code from the pull request

### Dependabot Merge Validation

- When configuring automatic merging or making exceptions for Dependabot, validate the **user** not the actor:
  - Use `github.event.pull_request.user.login == 'dependabot[bot]'`
  - Do **not** use `github.actor == 'dependabot[bot]'`

### Secrets & Publishing

- Use [Trusted Publishing](https://docs.pypi.org/trusted-publishers/) when publishing packages from GitHub Actions
- Do not use caching in sensitive workflows to prevent cache poisoning
- Avoid using `GITHUB_ENV` and `GITHUB_PATH` to pass parameters between steps — use `GITHUB_OUTPUT` instead