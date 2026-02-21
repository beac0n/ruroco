{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/nixos-24.11.tar.gz") {} }:
let
  root = toString ./.;
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    cacert
    rustup
    openssl
    perl
    upx
    cargo-nextest
    cargo-tarpaulin
    clippy
    rustfmt
    fontconfig
  ];

  CARGO_HOME = "${root}/.nix-cargo-linux";
  RUST_BACKTRACE = 1;

  OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include/openssl";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  OPENSSL_ROOT_DIR = "${pkgs.openssl.out}";

  shellHook = ''
    export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
    export PATH=$CARGO_HOME/bin:$PATH
    export LD_LIBRARY_PATH=${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH
    rustup default stable
  '';

}
