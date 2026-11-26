# The anatomy declaration reads and checks with the pinned ethos-zero; a
# refusal is printed before the check fails.
set -eu
status=0
reply="$("$generator/bin/ethos-zero" "Check.[ «$declaration» ]" 2>&1)" || status=$?
case "$status:$reply" in
  0:Checked.*) touch "$out" ;;
  *) echo "$declaration (exit $status): $reply" >&2; exit 1 ;;
esac
