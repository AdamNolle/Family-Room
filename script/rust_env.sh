#!/usr/bin/env bash
# Homebrew cargo/rustc can precede rustup proxies. Pin the actual compiler too.
export PATH="$(dirname "$(rustup which --toolchain 1.98.0 cargo)"):$PATH"
export RUSTC="$(rustup which --toolchain 1.98.0 rustc)"
export RUSTDOC="$(rustup which --toolchain 1.98.0 rustdoc)"
