{
  description = "protos — universal structural substrate";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-build = {
      url = "github:LiGoldragon/rust-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-build }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rust = rust-build.lib.${system}.fromPkgs pkgs;
        inherit (rust) craneLib toolchain;
        src = rust.cleanSource { root = ./.; };
        common = { inherit src; strictDeps = true; cargoArtifacts = null; doInstallCargoArtifacts = false; };
      in {
        packages.default = craneLib.buildPackage common;
        checks = {
          build = craneLib.cargoBuild common;
          test = craneLib.cargoTest common;
          doc = craneLib.cargoDoc (common // { RUSTDOCFLAGS = "-D warnings"; });
          fmt = craneLib.cargoFmt { inherit src; doInstallCargoArtifacts = false; };
          clippy = craneLib.cargoClippy (common // { cargoClippyExtraArgs = "--all-targets -- -D warnings"; });
        };
        devShells.default = pkgs.mkShell { packages = [ pkgs.jujutsu toolchain ]; };
      });
}
