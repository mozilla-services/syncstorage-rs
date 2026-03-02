
# Creating Releases

1. Switch to master branch of syncstorage-rs
1. `git pull` to ensure that the local copy is up-to-date.
1. `git pull origin master` to make sure that you've incorporated any changes to the master branch.
1. `git diff origin/master` to ensure that there are no local staged or uncommited changes.
1. Bump the version number in [Cargo.toml](https://github.com/mozilla-services/syncstorage-rs/blob/master/Cargo.toml) (this new version number will be designated as `<version>` in this checklist)
1. create a git branch for the new version `git checkout -b release/<version>`
1. `cargo build --release` - Build with the release profile [release mode](https://doc.rust-lang.org/book/ch14-01-release-profiles.html).
1. `clog -C CHANGELOG.md` - Generate release notes. We're using [clog](https://github.com/clog-tool/clog-cli) for release notes. Add a `-p`, `-m` or `-M` flag to denote major/minor/patch version, ie `clog -C CHANGELOG.md -p`.
1. Review the `CHANGELOG.md` file and ensure all relevant changes since the last tag are included.
1. Create a new [release in Sentry](https://docs.sentry.io/product/releases/#create-release): `VERSION={release-version-here} bash scripts/sentry-release.sh`. If you're doing this for the first time, checkout the [tips below](https://github.com/mozilla-services/syncstorage-rs#troubleshooting) for troubleshooting sentry cli access.
1. `git commit -am "chore: tag <version>"` to commit the new version and changes
1. `git tag -s -m "chore: tag <version>" <version>` to create a signed tag of the current HEAD commit for release.
1. `git push origin release/<version>` to push the commits to a new origin release branch
1. `git push --tags origin release/<version>` to push the tags to the release branch.
1. Submit a Pull Request (PR) on github to merge the release branch to master.
1. Go to the [GitHub release](https://github.com/mozilla-services/syncstorage-rs/releases), you should see the new tag with no release information.
1. Click the `Draft a new release` button.
1. Enter the \<version> number for `Tag version`.
1. Copy and paste the most recent change set from `CHANGELOG.md` into the release description, omitting the top 2 lines (the name and version)
1. Once your PR merges, click [Publish Release] on the [GitHub release](https://github.com/mozilla-services/syncstorage-rs/releases) page.

Sync server is automatically deployed to STAGE, however QA may need to be notified if testing is required. Once QA signs off, then a bug should be filed to promote the server to PRODUCTION.