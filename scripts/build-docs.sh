#!/bin/sh

# Copied from: https://github.com/kmcallister/travis-doc-upload
# License: CC0 1.0 Universal
# https://creativecommons.org/publicdomain/zero/1.0/legalcode

set -e

echo "Branch: $TRAVIS_BRANCH    Pull request: $TRAVIS_PULL_REQUEST"

if [ "$TRAVIS_BRANCH" = "master" -a "$TRAVIS_PULL_REQUEST" = "false" ]; then
  echo "Building docs for deployment."
  cargo doc --document-private-items

  eval key=\$encrypted_ad173ee39c4b_key
  eval iv=\$encrypted_ad173ee39c4b_iv

  mkdir -p ~/.ssh
  openssl aes-256-cbc -K $key -iv $iv -in scripts/id_rsa.enc -out ~/.ssh/id_rsa -d
  chmod 600 ~/.ssh/id_rsa

  git clone --branch gh-pages git@github.com:mozilla-services/syncstorage-rs.git docs
  cd docs
  rm -rf *
  mv ../target/doc/* .

  CHANGES=`git status --porcelain`

  if [ "$CHANGES" = "" ]; then
    echo "Docs are unchanged, not deploying to GitHub Pages."
    exit 0
  fi

  echo "Deploying docs to GitHub Pages."

  git config user.name "app-services-devs"
  git config user.email "application-services@mozilla.com"
  git add -A .
  git commit -qm "chore: rebuild developer docs"
  git push -q origin gh-pages
else
  echo "Not building docs for deployment, omitting dependencies."
  cargo doc --document-private-items --no-deps
fi
