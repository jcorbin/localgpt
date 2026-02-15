#!/bin/bash
set -e
set -x

umask 0002

[ -d update_prefix ] || mkdir update_prefix
cargo install --root="$(pwd)/update_prefix" --path=./crates/cli

sys_prog=localgpt-$(git describe --tags --always)
sys_bin="/usr/local/bin/$sys_prog"

cp update_prefix/bin/localgpt "$sys_bin"
ln -nsf "$sys_prog" /usr/local/bin/localgpt
