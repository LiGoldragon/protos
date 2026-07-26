{
  description = "protos — implementation-free component contracts";

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
        commonArguments = { inherit src; strictDeps = true; };
        checkArguments = commonArguments // {
          cargoArtifacts = null;
          doInstallCargoArtifacts = false;
        };
        package = craneLib.buildPackage (commonArguments // {
          cargoArtifacts = null;
          doInstallCargoArtifacts = false;
          installPhaseCommand = ''
            artifact_directory=target/release/deps
            test -d "$artifact_directory"
            install -d -m 755 "$out/lib"

            find "$artifact_directory" -maxdepth 1 -type f \
              \( -name 'lib*.rlib' -o -name 'lib*.rmeta' \) -print \
              | sort \
              | while IFS= read -r artifact; do
                  install -Dm444 "$artifact" "$out/lib/$(basename "$artifact")"
                done

            protos_rlib="$(find "$out/lib" -maxdepth 1 -type f -name 'libprotos-*.rlib' -print -quit)"
            protos_rmeta="$(find "$out/lib" -maxdepth 1 -type f -name 'libprotos-*.rmeta' -print -quit)"
            test -n "$protos_rlib"
            test -n "$protos_rmeta"
            install -Dm444 "$protos_rlib" "$out/lib/libprotos.rlib"
            install -Dm444 "$protos_rmeta" "$out/lib/libprotos.rmeta"
          '';
        });
      in
      {
        packages.default = package;
        checks = {
          build = craneLib.cargoBuild checkArguments;
          test = craneLib.cargoTest checkArguments;
          doc = craneLib.cargoDoc (checkArguments // {
            RUSTDOCFLAGS = "-D warnings";
          });
          fmt = craneLib.cargoFmt {
            inherit src;
            doInstallCargoArtifacts = false;
          };
          clippy = craneLib.cargoClippy (checkArguments // {
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });
          package-contents = pkgs.runCommand "protos-package-contents" {
            nativeBuildInputs = [ pkgs.findutils pkgs.coreutils ];
          } ''
            test -s ${package}/lib/libprotos.rlib
            test -s ${package}/lib/libprotos.rmeta
            test -n "$(find ${package}/lib -maxdepth 1 -type f -name 'libprotos-*.rlib' -print -quit)"
            touch $out
          '';
        };
        devShells.default = pkgs.mkShell {
          name = "protos";
          packages = [ pkgs.jujutsu toolchain ];
        };
      });
}
