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
        src = rust.cleanSource {
          root = ./.;
          extraFilters = [
            (path: type: pkgs.lib.hasInfix "/checks" (toString path))
          ];
        };
        common = { inherit src; strictDeps = true; cargoArtifacts = null; doInstallCargoArtifacts = false; };
        guardManifest = "checks/architecture-guards/Cargo.toml";
        guardTest = testName: craneLib.cargoTest (common // {
          cargoTestExtraArgs = "--manifest-path ${guardManifest} --test guards ${testName} -- --exact";
        });
      in {
        packages.default = craneLib.buildPackage common;
        checks = {
          build = craneLib.cargoBuild common;
          test = craneLib.cargoTest common;
          architecture-guards = craneLib.cargoTest (common // {
            cargoTestExtraArgs = "--manifest-path ${guardManifest} --test guards";
          });
          no-production-free-functions = guardTest "no_production_free_functions_fixture";
          no-production-inherent-methods = guardTest "no_production_inherent_methods_fixture";
          no-zst-behavior = guardTest "no_zst_behavior_fixture";
          no-forbidden-vocabulary = guardTest "no_forbidden_vocabulary_fixture";
          doc = craneLib.cargoDoc (common // { RUSTDOCFLAGS = "-D warnings"; });
          fmt = craneLib.cargoFmt { inherit src; doInstallCargoArtifacts = false; };
          clippy = craneLib.cargoClippy (common // { cargoClippyExtraArgs = "--all-targets -- -D warnings"; });
        };
        devShells.default = pkgs.mkShell { packages = [ pkgs.jujutsu toolchain ]; };
      });
}
