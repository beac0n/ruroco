#!/bin/sh

USAGE="Usage: $0 {major|minor|patch}"

if [ "$#" -ne 1 ]; then
    printf "%s\n" "$USAGE"
    exit 1
elif [ ! -f VERSION ]; then
    VERSION="0.0.0"
else
    VERSION=$(cat VERSION)
    VERSION=${VERSION#v}
fi

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
printf "%s" "$NEW_VERSION" > VERSION
printf "Updated version to %s\n" "$NEW_VERSION"
