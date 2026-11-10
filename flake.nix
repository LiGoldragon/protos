{
  description = "protos — universal structural substrate";

  inputs = {
    base.url = "github:LiGoldragon/build-base";
    ethos-zero = {
      url = "github:LiGoldragon/ethos-zero";
      inputs.base.follows = "base";
    };
  };

  # Cargo.nix is generated from Cargo.lock by the build base's crate2nix and
  # committed, so evaluation imports no derivation:
  #   nix run github:LiGoldragon/build-base#crate2nix -- generate
  outputs =
    { self, base, ethos-zero }:
    base.lib.workspaceOutputs {
      cargoNix = ./Cargo.nix;

      # Every test runs under the rkyv feature, the archival test with them.
      memberChecks = { check, ... }: { protos = check "protos" [ "default" "rkyv" ]; };

      extraChecks =
        { pkgs, toolchain, doc, ... }:
        let
          source = ./src;
          generator = ethos-zero.packages.${pkgs.stdenv.hostPlatform.system}.default;
          manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          # The Rust sources, tests and the generated module a test includes.
          rust = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [ ./src ./tests ./generated ];
          };
        in
        {
          no-production-free-functions = pkgs.runCommand "protos-no-production-free-functions" { } ''
            if grep -R -n -E '^(pub(\([^)]*\))? )?fn ' ${source}; then
              echo "production Rust must not use module-level free functions" >&2
              exit 1
            fi
            touch $out
          '';
          no-production-inherent-methods = pkgs.runCommand "protos-no-production-inherent-methods" { } ''
            if grep -R -n -E '^[[:space:]]*impl[[:space:]]+[[:alpha:]_][[:alnum:]_:<>]*[[:space:]]*\{' ${source}; then
              echo "production Rust must home behavior in traits" >&2
              exit 1
            fi
            touch $out
          '';
          no-zst-behavior = pkgs.runCommand "protos-no-zst-behavior" { } ''
            if grep -R -n -E '^[[:space:]]*(pub[[:space:]]+)?struct[[:space:]]+[[:alpha:]_][[:alnum:]_]*[[:space:]]*;' ${source}; then
              echo "behavioral Rust nouns must carry data" >&2
              exit 1
            fi
            touch $out
          '';
          no-forbidden-vocabulary = pkgs.runCommand "protos-no-forbidden-vocabulary" { } ''
            if grep -R -n -i -E 'encode|decode|codec|transcode' ${source}; then
              echo "Protos names must use the ruled form vocabulary" >&2
              exit 1
            fi
            touch $out
          '';
          generated-kinds = pkgs.runCommand "protos-generated-kinds" {
            inherit generator;
            declaration = ./protos-kinds.ethos;
            committed = ./generated/protos-kinds.rs;
          } (builtins.readFile ./checks/generated-kinds.sh);
          checked-anatomy = pkgs.runCommand "protos-checked-anatomy" {
            inherit generator;
            declaration = ./protos.ethos;
          } (builtins.readFile ./checks/checked-anatomy.sh);
          fmt = pkgs.runCommand "protos-fmt" { nativeBuildInputs = [ toolchain ]; } ''
            rustfmt --check --edition ${manifest.package.edition} ${rust}/src/lib.rs ${rust}/tests/*.rs
            touch $out
          '';
          # rustdoc over the library with the rkyv feature, warnings refused.
          doc = doc "protos" [ "default" "rkyv" ] [ "-D" "warnings" ];
        };
    };
}
