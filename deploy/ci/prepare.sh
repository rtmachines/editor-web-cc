#!/usr/bin/env bash
# D11 steps 1 and 2. Each path belongs to this runner job.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

sudo apt-get update -qq
sudo apt-get install --yes --no-install-recommends acl
[[ ! -e $CI_ROOT ]] || fail "the runner directory already exists"
mkdir -m 0700 -- "$CI_ROOT" "$CI_STATE"
for disk in backups walarchive
do
  truncate -s 4G "$CI_ROOT/$disk.img"
  mkfs.ext4 -q -F "$CI_ROOT/$disk.img"
  mkdir -m 0700 -- "$CI_ROOT/$disk"
  sudo mount -o loop,nodev,nosuid "$CI_ROOT/$disk.img" "$CI_ROOT/$disk"
  sudo chown 999:999 "$CI_ROOT/$disk"
done
sudo chmod 0755 "$RECIPE_BACKUP_DIR"
sudo chmod 0700 "$RECIPE_WALARCHIVE_DIR"
sudo install -o 999 -g 999 -m 0600 /dev/null "$RECIPE_BACKUP_DIR/.backup-marker"
sudo install -o 999 -g 999 -m 0600 /dev/null "$RECIPE_WALARCHIVE_DIR/.walarchive-marker"
sudo install -d -o root -g root -m 0700 "$RECIPE_SECRETS_DIR"
for name in postgres_password migrator_password app_password backup_password replicator_password login_hmac_key csrf_signing_key
do
  openssl rand -hex 32 | sudo tee "$RECIPE_SECRETS_DIR/$name" >/dev/null
  sudo chmod 0400 "$RECIPE_SECRETS_DIR/$name"
  case $name in
    login_hmac_key|csrf_signing_key) sudo chown 65532:65532 "$RECIPE_SECRETS_DIR/$name" ;;
    *) sudo chown 999:999 "$RECIPE_SECRETS_DIR/$name" ;;
  esac
done
for name in app_password migrator_password
do
  sudo setfacl -m u:999:r,u:65532:r,g::-,m::r "$RECIPE_SECRETS_DIR/$name"
done
sudo install -o 65532 -g 65532 -m 0400 /dev/null "$RECIPE_SECRETS_DIR/alerts_webhook_url"
openssl rand -hex 32 >"$CI_STATE/admin_password"
openssl rand -hex 32 >"$CI_STATE/new_password"
sudo install -o 65532 -g 65532 -m 0400 "$CI_STATE/admin_password" "$RECIPE_SECRETS_DIR/admin_password"
printf 'CI disks and secrets are ready.\n'
