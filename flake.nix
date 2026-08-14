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
        guardScript = ./checks/architecture-guards.py;
        guardFixtures = ./checks/fixtures/architecture-guards;
        guardCheck = guard: name: pkgs.runCommand name { nativeBuildInputs = [ pkgs.python3 ]; } ''
          ${pkgs.python3}/bin/python ${guardScript} ${src}/src ${guardFixtures} --guard ${guard}
          touch $out
        '';
      in {
        packages.default = craneLib.buildPackage common;
        checks = {
          build = craneLib.cargoBuild common;
          test = craneLib.cargoTest common;
          architecture-guards = pkgs.runCommand "protos-architecture-guards" { nativeBuildInputs = [ pkgs.python3 ]; } ''
            ${pkgs.python3}/bin/python ${guardScript} ${src}/src ${guardFixtures}
            touch $out
          '';
          no-production-free-functions = guardCheck "free-functions" "protos-no-production-free-functions";
          no-production-inherent-methods = guardCheck "inherent-methods" "protos-no-production-inherent-methods";
          no-zst-behavior = guardCheck "zst-behavior" "protos-no-zst-behavior";
          no-forbidden-vocabulary = guardCheck "forbidden-vocabulary" "protos-no-forbidden-vocabulary";
          doc = craneLib.cargoDoc (common // { RUSTDOCFLAGS = "-D warnings"; });
          fmt = craneLib.cargoFmt { inherit src; doInstallCargoArtifacts = false; };
          clippy = craneLib.cargoClippy (common // { cargoClippyExtraArgs = "--all-targets -- -D warnings"; });
        };
        devShells.default = pkgs.mkShell { packages = [ pkgs.jujutsu toolchain ]; };
      });
}
