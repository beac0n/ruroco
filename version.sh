#!/bin/sh

USAGE="Usage: $0 {major|minor|patch}"

if [ "$#" -ne 1 ]; then
    printf "%s\n" "$USAGE"
    exit 1

VERSION=$(grep '^version = ' Cargo.toml | cut -d '"' -f 2)
MAJOR=$(printf "%s" "$VERSION" | cut -d '.' -f 1)
MINOR=$(printf "%s" "$VERSION" | cut -d '.' -f 2)
PATCH=$(printf "%s" "$VERSION" | cut -d '.' -f 3)

case "$1" in
    major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0;;
    minor) MINOR=$((MINOR + 1)); PATCH=0;;
    patch) PATCH=$((PATCH + 1));;
    *) printf "%s\n" "$USAGE"; exit 1;;
esac

NEW_VERSION="v${MAJOR}.${MINOR}.${PATCH}"
NEW_VERSION_SEMVER=${NEW_VERSION#v}

sed -i -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$NEW_VERSION_SEMVER\"/" Cargo.toml
git cliff --unreleased --tag "$NEW_VERSION_SEMVER" --prepend CHANGELOG.md
printf "Updated version to %s\n" "$NEW_VERSION"
cargo update

git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Bump version to ${NEW_VERSION}"
git tag "$NEW_VERSION"

printf "Created new commit and tag to %s - push with git push && git push --tags\n" "$NEW_VERSION"