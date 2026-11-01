{
  description = "rustybottles, a utility for managing Bottles installations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";

    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ (import rust-overlay) ];
        };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };
      in {
        packages.default = rustPlatform.buildRustPackage {
          pname = "rustybottles";
          version = "0.1.0";

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.unrar ];

          meta = {
            description = "A utility for managing Bottles installations";
            mainProgram = "rustybottles";
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [
            rust
            pkgs.rust-analyzer
            pkgs.pkg-config
          ];

          buildInputs = [ pkgs.unrar ];
        };
      });
}
