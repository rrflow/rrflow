#!/usr/bin/env bash
# Clone committed history from a read-only checkout mount into the empty Linux
# source volume, then restore the checkout's remote names and URLs.
#
# Credentials, hooks, local Git configuration, and uncommitted files are never
# copied, and an existing workspace is never overwritten.
set -euo pipefail

seed="${1:-/seed}"
destination="${2:-/workspaces/rrflow}"

fail() {
  echo "seed-workspace: $*" >&2
  exit 1
}

git -C "$seed" rev-parse --git-dir >/dev/null 2>&1 \
  || fail "no Git repository at $seed"
if [ -e "$destination" ] && [ -n "$(ls -A "$destination")" ]; then
  fail "$destination is not empty; refusing to overwrite existing work"
fi
branch="$(git -C "$seed" symbolic-ref --quiet --short HEAD)" \
  || fail "the seed checkout has a detached HEAD; check out a branch first"

git clone --no-hardlinks --origin seed --branch "$branch" "$seed" "$destination"

while read -r remote; do
  git -C "$destination" remote add "$remote" "$(git -C "$seed" remote get-url "$remote")"
done < <(git -C "$seed" remote)

# Carry the seed's remote-tracking refs so upstream comparisons work before the
# first authenticated fetch, then drop the temporary seed remote.
git -C "$destination" fetch --no-tags seed '+refs/remotes/*:refs/remotes/*'
git -C "$destination" remote remove seed

upstream_remote="$(git -C "$seed" config --get "branch.$branch.remote" || true)"
upstream_merge="$(git -C "$seed" config --get "branch.$branch.merge" || true)"
if [ -n "$upstream_remote" ] && [ -n "$upstream_merge" ]; then
  git -C "$destination" config "branch.$branch.remote" "$upstream_remote"
  git -C "$destination" config "branch.$branch.merge" "$upstream_merge"
fi

# Declared submodules are pinned public HTTPS source inputs; materialize the
# exact gitlink revisions so the workspace matches a recursive CI checkout.
if [ -f "$destination/.gitmodules" ]; then
  git -C "$destination" submodule update --init --recursive
fi

echo "seed-workspace: cloned $branch at $(git -C "$destination" rev-parse --short HEAD) into $destination"
git -C "$destination" remote -v
