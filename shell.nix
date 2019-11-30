# Nix is a powerful package manager for Linux and other Unix systems that makes
# package management reliable and reproducible: https://nixos.org/nix/.
# This file is intended to be used with `nix-shell`
# (https://nixos.org/nix/manual/#sec-nix-shell) to setup a fully-functional
# syncstorage-rs build environment by installing all required dependencies.
with import <nixpkgs> {};
stdenv.mkDerivation {
  name = "syncstorage-rs";
  buildInputs = [
    rustc
    cargo
    libmysqlclient
    pkgconfig
    openssl
    cmake
    protobuf
    go
  ];
  NIX_LDFLAGS = "-L${libmysqlclient}/lib/mysql";
}
