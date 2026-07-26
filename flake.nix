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
        rustDylibExtension = pkgs.stdenv.hostPlatform.extensions.sharedLibrary;
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
              \( -name 'lib*.rlib' -o -name 'lib*.rmeta' \
                 -o -name "lib*${rustDylibExtension}" \) -print \
              | sort \
              | while IFS= read -r artifact; do
                  install -Dm444 "$artifact" "$out/lib/$(basename "$artifact")"
                done

            mapfile -t protos_rlibs < <(
              find "$out/lib" -maxdepth 1 -type f -name 'libprotos-*.rlib' -print | sort
            )
            mapfile -t protos_rmetas < <(
              find "$out/lib" -maxdepth 1 -type f -name 'libprotos-*.rmeta' -print | sort
            )
            test "''${#protos_rlibs[@]}" -eq 1
            test "''${#protos_rmetas[@]}" -eq 1
            protos_rlib="''${protos_rlibs[0]}"
            protos_rmeta="''${protos_rmetas[0]}"
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
            nativeBuildInputs = [ pkgs.findutils pkgs.coreutils pkgs.stdenv.cc toolchain ];
          } ''
            test -s ${package}/lib/libprotos.rlib
            test -s ${package}/lib/libprotos.rmeta
            protos_rlib_count="$(find ${package}/lib -maxdepth 1 -type f -name 'libprotos-*.rlib' -printf . | wc -c)"
            protos_rmeta_count="$(find ${package}/lib -maxdepth 1 -type f -name 'libprotos-*.rmeta' -printf . | wc -c)"
            test "$protos_rlib_count" -eq 1
            test "$protos_rmeta_count" -eq 1
            rustc \
              --edition=2024 \
              --crate-name protos_package_consumer \
              --crate-type bin \
              -L dependency=${package}/lib \
              --extern protos=${package}/lib/libprotos.rlib \
              -o protos-package-consumer \
              - <<'EOF'
            fn main() {
                assert_eq!(protos::SIGNAL_SPIRIT_CONTRACT_ID.value(), 1);
                assert_eq!(protos::META_SIGNAL_SPIRIT_CONTRACT_ID.value(), 2);
                assert_eq!(protos::SIGNAL_SPIRIT_JUDGE_CONTRACT_ID.value(), 3);
                assert_eq!(protos::SIGNAL_SPIRIT_JUDGE_WIRE_REVISION.value(), 1);
                assert_eq!(
                    protos::WireContractFamily::SignalSpiritJudge.current_binding(),
                    Some(protos::SIGNAL_SPIRIT_JUDGE_BINDING)
                );
                assert_eq!(protos::WireContractFamily::COUNT, 3);
            }
            EOF
            ./protos-package-consumer
            touch $out
          '';
        };
        devShells.default = pkgs.mkShell {
          name = "protos";
          packages = [ pkgs.jujutsu toolchain ];
        };
      });
}
