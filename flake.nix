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
        cargoToml = with builtins; (fromTOML (readFile ./Cargo.toml));

        inherit (pkgs) lib;
      in {
        packages.erg = pkgs.rustPlatform.buildRustPackage {
          inherit (cargoToml.package) name version;
          src = builtins.path {
            path = ./.;
            filter = name: type:
              (name == toString ./Cargo.toml)
              || (name == toString ./Cargo.lock)
              || (lib.hasPrefix (toString ./compiler) name)
              || (lib.hasPrefix (toString ./src) name);
          };
          cargoLock.lockFile = ./Cargo.lock;
        };
        packages.default = self.packages.${system}.erg;

        devShells.default = pkgs.devshell.mkShell {
          packages = with pkgs; [
            gcc
            rustc
            cargo
            # Dev
            python3
            treefmt # cli to run all formatters
            alejandra # Nix formatter
            # rustfmt # Rust Formatter
            # taplo-cli # TOML formatter
          ];
        };

        checks = {
          erg = self.packages.${system}.erg;
        };
      }
    );
}
