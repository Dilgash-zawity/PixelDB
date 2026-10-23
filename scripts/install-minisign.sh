#!/usr/bin/env bash
set -euo pipefail

minisign_dir=$(mktemp -d)
trap 'rm -rf "$minisign_dir"' EXIT

curl --fail --location --retry 3 \
  https://github.com/jedisct1/minisign/releases/download/0.12/minisign-0.12-linux.tar.gz \
  --output "$minisign_dir/minisign-0.12-linux.tar.gz"
printf '%s  %s\n' \
  '9a599b48ba6eb7b1e80f12f36b94ceca7c00b7a5173c95c3efc88d9822957e73' \
  "$minisign_dir/minisign-0.12-linux.tar.gz" | sha256sum --check
tar -xzf "$minisign_dir/minisign-0.12-linux.tar.gz" \
  -C "$minisign_dir" minisign-linux/x86_64/minisign
sudo install -m 755 "$minisign_dir/minisign-linux/x86_64/minisign" /usr/local/bin/minisign
