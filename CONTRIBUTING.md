# Contribution Guidelines

Anyone is welcome to contribute to this project. Feel free to get in touch with
other community members on IRC, the mailing list or through issues here on
GitHub.

[See the README](/README.md) for contact information.

## Bug Reports

You can file issues here on GitHub. Please try to include as much information as
you can and under what conditions you saw the issue.

## Sending Pull Requests

Patches should be submitted as pull requests (PR).

Before submitting a PR:
- Your code must run and pass all the automated tests before you submit your PR
  for review. "Work in progress" pull requests are allowed to be submitted, but
  should be clearly labeled as such and should not be merged until all tests
  pass and the code has been reviewed.
- Your patch should include new tests that cover your changes. It is your and
  your reviewer's responsibility to ensure your patch includes adequate tests.

When submitting a PR:
- You agree to license your code under the project's open source license
  ([MPL 2.0](/LICENSE)).
- Base your branch off the current `master`.
- Add both your code and new tests if relevant.
- Sign your git commit.
- Run the test suite to make sure your code passes linting and tests.
- Ensure your changes do not reduce code coverage of the test suite.
- Please do not include merge commits in pull requests; include only commits
  with the new relevant code.
- PR naming conventions - begins with type (fix, feature, doc, chore, etc) and a short description with no period.

See the main [README.md](/README.md) for information on prerequisites,
installing, running and testing.

## Code Review

This project is production Mozilla code and subject to our [engineering practices and quality standards](https://developer.mozilla.org/en-US/docs/Mozilla/Developer_guide/Committing_Rules_and_Responsibilities). Every patch must be peer reviewed.

## Git Commit Guidelines

We loosely follow the [Angular commit guidelines](https://github.com/angular/angular.js/blob/master/CONTRIBUTING.md#type)
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
