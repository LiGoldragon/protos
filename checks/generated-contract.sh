set -eu

temporary="$(mktemp -d)"
cleanup() {
  rm -rf "$temporary"
}
trap cleanup EXIT

name="$(basename "$declaration" .ethos)"
"$generator/bin/ethos-zero" "Generate.{ $declaration $temporary }"
cmp "$temporary/$name.rs" "$committed"
touch "$out"
