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
          no-production-free-functions = pkgs.runCommand "protos-no-production-free-functions" { } ''
            if grep -R -n -E '^(pub(\\([^)]*\\))? )?fn ' ${src}/src; then
              echo "production Rust must not use module-level free functions" >&2
              exit 1
            fi
            touch $out
          '';
          no-production-inherent-methods = pkgs.runCommand "protos-no-production-inherent-methods" { } ''
            if grep -R -n -E '^[[:space:]]*impl[[:space:]]+[[:alpha:]_][[:alnum:]_:<>]*[[:space:]]*\\{' ${src}/src; then
              echo "production Rust must home behavior in traits" >&2
              exit 1
            fi
            touch $out
          '';
          no-zst-behavior = pkgs.runCommand "protos-no-zst-behavior" { } ''
            if grep -R -n -E '^[[:space:]]*(pub[[:space:]]+)?struct[[:space:]]+[[:alpha:]_][[:alnum:]_]*[[:space:]]*;' ${src}/src; then
              echo "behavioral Rust nouns must carry data" >&2
              exit 1
            fi
            touch $out
          '';
          no-forbidden-vocabulary = pkgs.runCommand "protos-no-forbidden-vocabulary" { } ''
            if grep -R -n -i -E 'archive|code|encode|decode|codec|transcode' ${src}/src; then
              echo "Protos names must use the ruled form vocabulary" >&2
              exit 1
            fi
            touch $out
          '';
          doc = craneLib.cargoDoc (common // { RUSTDOCFLAGS = "-D warnings"; });
          fmt = craneLib.cargoFmt { inherit src; doInstallCargoArtifacts = false; };
          clippy = craneLib.cargoClippy (common // { cargoClippyExtraArgs = "--all-targets -- -D warnings"; });
        };
        devShells.default = pkgs.mkShell { packages = [ pkgs.jujutsu toolchain ]; };
      });
}
