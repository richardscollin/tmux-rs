{
  description = "A Rust port of tmux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;

        # use latest stable Rust compiler
        craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.latest.default);

        commonArgs = {
          # include Rust sources, and anything else required by build
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              (lib.fileset.fileFilter (file: file.hasExt "lalrpop") ./.)
              ./README.md
            ];
          };
          strictDeps = true;
        };

        tmux-rs = craneLib.buildPackage (
          commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
            cargoExtraArgs = "--verbose";

            # libraries we link with (per build.rs)
            buildInputs = [
              pkgs.libevent
              pkgs.ncurses
            ];
          }
        );
      in
      {
        checks = {
          inherit tmux-rs;
        };

        packages.default = tmux-rs;
        apps.default = flake-utils.lib.mkApp {
          drv = tmux-rs;
        };
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            pkgs.bashInteractive # full-featured bash with readline
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
          ];
        };
      }
    );
}
