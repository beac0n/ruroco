{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    androidenv.androidPkgs.androidsdk
    rustup
    openssl
    curl
    jdk17
    perl
  ];

  ANDROID_HOME="${pkgs.androidenv.androidPkgs.androidsdk}/libexec/android-sdk";
  ANDROID_NDK = "${pkgs.androidenv.androidPkgs.androidsdk}/libexec/android-sdk/ndk-bundle";
  OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include/openssl"
  OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
  OPENSSL_ROOT_DIR="${pkgs.openssl.out}"

  shellHook = ''
    rustup default stable
    export PATH=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/:$PATH
  '';

}
