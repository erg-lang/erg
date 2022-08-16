{
  description = "Nix Flake for Erg Programming Language";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    # develop
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };
  outputs = {
    self,
    nixpkgs,
    devshell,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            devshell.overlay
          ];
        };
      in {
        packages.default = let
          cargoToml = with builtins; (fromTOML (readFile ./Cargo.toml));
        in
          pkgs.rustPlatform.buildRustPackage {
            inherit (cargoToml.package) name version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
          };

        devShells.default = pkgs.devshell.mkShell {
          packages = with pkgs; [
            gcc
            rustc
            cargo
            rustfmt
            alejandra # Nix formatter
          ];
        };
      }
    );
}
