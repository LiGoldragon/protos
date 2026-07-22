{
  description = "protos — the shared structural machinery of the NOTA/Core language family: content identity, the name table, raw discovery, the structural-form kernel, and its derive";

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
        commonArguments = {
          inherit src;
          strictDeps = true;
          cargoExtraArgs = "--workspace";
        };
        cargoArtifacts = craneLib.buildDepsOnly (commonArguments // {
          # rustc writes this host-specific probe beside reusable artifacts.
          postCheck = ''
            rm -f target/.rustc_info.json
          '';
        });
      in
      {
        packages.default = craneLib.buildPackage (commonArguments // { inherit cargoArtifacts; });
        checks = {
          build = craneLib.cargoBuild (commonArguments // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
          });
          test = craneLib.cargoTest (commonArguments // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
          });
          doc = craneLib.cargoDoc (commonArguments // {
            inherit cargoArtifacts;
            CARGO_BUILD_JOBS = "1";
            RUSTDOCFLAGS = "-D warnings";
          });
          fmt = craneLib.cargoFmt { inherit src; };
          clippy = craneLib.cargoClippy (commonArguments // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });
        };
        devShells.default = pkgs.mkShell {
          name = "protos";
          packages = [ pkgs.jujutsu toolchain ];
        };
      });
}
