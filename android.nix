{ pkgs ? import <nixpkgs> {
    config = {
      android_sdk.accept_license = true;
      allowUnfree = true;
    };
} }:

let
  root = toString ./.;
  arch = pkgs.stdenv.hostPlatform.parsed.cpu.name;
  androidHome = "${androidComposition.androidsdk}/libexec/android-sdk";
  androidComposition = pkgs.androidenv.composeAndroidPackages {
    cmdLineToolsVersion = "9.0";
    toolsVersion = null;
    platformToolsVersion = "35.0.2";
    buildToolsVersions =  [ "35.0.1"];
    includeEmulator = false;
    includeCmake = false;
    cmakeVersions = [];
    includeNDK = true;
    ndkVersions = [ "28" ];
    includeExtras = [ ];
    platformVersions = [ "35x"];
    systemImageTypes = [ ];
    abiVersions = [ "arm64-v8a" ];
  };

in pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup
    androidComposition.androidsdk
    jdk23_headless
    openssl
    perl
    curl
    cargo-xbuild
  ];

  ANDROID_HOME = "${androidHome}";
  ANDROID_NDK_ROOT = "${androidHome}/ndk-bundle";

  CARGO_HOME = "${root}/.nix-cargo-android";

  OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include/openssl";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  OPENSSL_ROOT_DIR = "${pkgs.openssl.out}";

  shellHook = ''
    rustup default stable
    export PATH=$CARGO_HOME/bin:$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-${arch}/bin/:$PATH
  '';

}
