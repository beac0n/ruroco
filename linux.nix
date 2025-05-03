{ pkgs ? import <nixpkgs> {} }:
let
  root = toString ./.;
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup
    openssl
    upx
    perl
    cargo-nextest
    cargo-tarpaulin
    clippy
    rustfmt
  ];

  CARGO_HOME = "${root}/.nix-cargo-linux";

  OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include/openssl";
  OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib";
  OPENSSL_ROOT_DIR="${pkgs.openssl.out}";

  shellHook = ''
    export DISPLAY=:0
    export PATH=$CARGO_HOME/bin:$PATH
  '';

}
