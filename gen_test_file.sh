#!/usr/bin/env bash

rm -rf target/release/foo
mkdir -p target/release/foo

truncate -s 1M target/release/foo/1M.bin
truncate -s 10M target/release/foo/10M.bin
truncate -s 100M target/release/foo/100M.bin
truncate -s 1G target/release/foo/1G.bin
truncate -s 2G target/release/foo/10G.bin
