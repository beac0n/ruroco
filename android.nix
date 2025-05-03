{ pkgs ? import <nixpkgs> {} }:

let
  root = toString ./.;
  androidHome="${pkgs.androidenv.androidPkgs.androidsdk}/libexec/android-sdk";
  arch=pkgs.stdenv.hostPlatform.parsed.cpu.name;
in pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup  # xbuild needs rustup
    androidenv.androidPkgs.androidsdk  # for building android
    jdk23_headless # for building android
    openssl
    perl  # building openSSL requires perl
    curl  # needed for downloading skia libraries
  ];

  ANDROID_HOME="${androidHome}";
  ANDROID_NDK_ROOT="${androidHome}/ndk-bundle";

  CARGO_HOME="${root}/.nix-cargo";

  OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include/openssl";
  OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib";
  OPENSSL_ROOT_DIR="${pkgs.openssl.out}";

  shellHook = ''
    rustup default stable
    export PATH=$CARGO_HOME/bin:$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-${arch}/bin/:$PATH
  '';

}
