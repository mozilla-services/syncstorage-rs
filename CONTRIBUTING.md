# Contribution Guidelines

Anyone is welcome to contribute to this project. Feel free to get in touch with
other community members on IRC, the mailing list or through issues here on
GitHub.

[See the README](/README.md) for contact information.

## Bug Reports

You can file issues here on GitHub. Please try to include as much information as
you can and under what conditions you saw the issue. We will do our best to triage
the issue as soon as possible.

## Sending Pull Requests

Patches should be submitted as pull requests (PR).

Before submitting a PR:
- Your code must run and pass all the automated tests before you submit your PR
  for review. "Work in progress" or "Draft" pull requests are allowed to be submitted,
  but should be clearly labeled as such and should not be merged until all tests pass and the code
  has been reviewed.
- Your patch should include new tests that cover your changes. It is your and
  your reviewer's responsibility to ensure your patch includes adequate tests.

When submitting a PR:
- **[Sign all your git commits](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification#ssh-commit-verification)**.
  We cannot accept any PR that does not have all commits signed. This is a policy
  put in place by our Security Operations team and is enforced by our CI processes.
- You agree to license your code under the project's open source license
  ([MPL 2.0](/LICENSE)).
- Base your branch off the current `master`.
- Add both your code and new tests if relevant.
- Run the test suite to make sure your code passes linting and tests.
- Ensure your changes do not reduce code coverage of the test suite.
- Please do not include merge commits in pull requests; include only commits
  with the new relevant code.
- PR naming conventions - begins with type (fix, feature, doc, chore, etc) and a short description with no period.

See the main [README.md](/README.md) for information on prerequisites,
installing, running and testing.

## Code Review

This project is production Mozilla code and subject to the contributing guidelines established in this documentation. Every patch must be peer reviewed by a member of the official Sync team. 

## Git Commit Guidelines

We loosely follow the [Angular commit guidelines][angular_commit_guidelines]
of `<type>: <subject>` where `type` must be one of:

* **feat**: A new feature
* **fix**: A bug fix
* **docs**: Documentation only changes
* **style**: Changes that do not affect the meaning of the code (white-space, formatting, missing
  semi-colons, etc)
* **refactor**: A code change that neither fixes a bug or adds a feature
* **perf**: A code change that improves performance
* **test**: Adding missing tests
* **chore**: Changes to the build process or auxiliary tools and libraries such as documentation
  generation

For Mozilla engineers:

If associated with a Jira ticket, synchronization with Jira and GitHub is possible by appending the suffix of the Jira ticket to the branch name (`STOR-1234` in the example below). Name the branch using the appropriate `<type>` above followed by a forward slash, followed by a dash-separated description of the task and then by the Jira ticket and 
. Ex. `feat/add-sentry-sdk-STOR-1234` or `add-sentry-sdk-STOR-1234` 

Note: the Jira ticket project and number can be added anywhere in the
branch name, but adding to the beginning is ideal. You can also include the Jira issue at the end of
commit messages to keep the task up to date. See Jira Docs for referencing issues [here][jira].

For Community Contributors:

You won't have access to Jira, so just add the GitHub Issue number at the end of the PR branch name:
Ex. `feat/add-sentry-sdk-1112`

For Everyone:
Make sure that the title of your pull request in GitHub has the `type` followed by a colon so that it will be automatically added in the changelog when Sync is published for release.
Ex. `feat: add sentry sdk`

[angular_commit_guidelines]: https://github.com/angular/angular/blob/main/CONTRIBUTING.md
[jira]: https://support.atlassian.com/jira-software-cloud/docs/reference-issues-in-your-development-work/

### Subject

The subject contains succinct description of the change:

* use the imperative, present tense: "change" not "changed" nor "changes"
* don't capitalize first letter
* no dot (.) at the end

### Body

In order to maintain a reference to the context of the commit, add
`Closes #<issue_number>` if it closes a related issue or `Issue #<issue_number>`
if it's a partial fix.

You can also write a detailed description of the commit: Just as in the
**subject**, use the imperative, present tense: "change" not "changed" nor
"changes" It should include the motivation for the change and contrast this with
previous behavior.

### Footer

The footer should contain any information about **Breaking Changes** and is also
the place to reference GitHub issues that this commit **Closes**.

### Example

A properly formatted commit message should look like:

```
feat: give the developers a delicious cookie

Properly formatted commit messages provide understandable history and
documentation. This patch will provide a delicious cookie when all tests have
passed and the commit message is properly formatted.

BREAKING CHANGE: This patch requires developer to lower expectations about
    what "delicious" and "cookie" may mean. Some sadness may result.

Closes #3.14, #9.75
```
