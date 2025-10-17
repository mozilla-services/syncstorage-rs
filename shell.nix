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
    pkg-config
    openssl
    cmake
    protobuf
    go
    grpc
  ];

  # grpc otherwise fails since it's bulit with `-Wall`
  hardeningDisable = [ "all" ];

  GRPCIO_SYS_USE_PKG_CONFIG = 1;

  NIX_LDFLAGS = "-L${libmysqlclient}/lib/mysql";
}
