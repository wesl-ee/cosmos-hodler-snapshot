let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
in
with nixpkgs;
stdenv.mkDerivation {
  name = "cosmos-hodler-snapshot";
  buildInputs = [
    # latest stable wasm toolchain
    (latest.rustChannels.stable.rust.override {
      targets = ["wasm32-unknown-unknown"];
    })
    latest.rustChannels.stable.rust-src
    git
  ];

  RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";
}
