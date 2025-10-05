{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/nixos-24.11.tar.gz") {
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
    buildToolsVersions =  [ "35.0.0"];
    includeEmulator = false;
    platformVersions = [ "35" ];
    includeSources = false;
    includeSystemImages = false;
    systemImageTypes = [  ];
    abiVersions = [ "arm64-v8a" ];
    cmakeVersions = [ ];
    includeNDK = true;
    ndkVersions = [ "27.0.12077973" ];
    useGoogleAPIs = false;
    useGoogleTVAddOns = false;
    includeExtras = [ ];
  };

in pkgs.mkShell {
  buildInputs = with pkgs; [
    cacert
    rustup
    openssl
    perl
    androidComposition.androidsdk
    jdk21 # must be the same as use in androidenv
    curlMinimal
  ];

  ANDROID_HOME = "${androidHome}";
  ANDROID_NDK_ROOT = "${androidHome}/ndk-bundle";

  CARGO_HOME = "${root}/.nix-cargo-android";
  RUST_BACKTRACE = 1;

  OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include/openssl";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  OPENSSL_ROOT_DIR = "${pkgs.openssl.out}";

  shellHook = ''
    export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
    export PATH=$CARGO_HOME/bin:$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-${arch}/bin/:$PATH
    export LD_LIBRARY_PATH=${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH
    rustup default stable
  '';

}
