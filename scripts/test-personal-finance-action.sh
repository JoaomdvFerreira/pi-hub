#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ACTION="$ROOT/scripts/trusted-actions/personal-finance"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
HEAD=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
REMOTE=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
LOG="$TMP/git.log"
mkdir -p "$TMP/repo/.git" "$TMP/bin"
cat >"$TMP/bin/git" <<'EOF'
#!/bin/sh
set -eu
printf '%s\n' "$*" >> "$PIHUB_TEST_LOG"
case "$*" in
  *'config --get branch.main.remote'*) printf '%s\n' origin;;
  *'config --get branch.main.merge'*) printf '%s\n' refs/heads/main;;
  *'status --porcelain'*) [ "${PIHUB_TEST_DIRTY:-0}" = 0 ];;
  *'rev-parse --verify HEAD^{commit}'*) printf '%s\n' "$PIHUB_TEST_HEAD";;
  *'ls-remote origin refs/heads/main'*)
    case "${PIHUB_TEST_REMOTE_MODE:-one}" in one) printf '%s\trefs/heads/main\n' "$PIHUB_TEST_REMOTE";; malformed) printf 'NOT_A_SHA\trefs/heads/main\n';; missing) :;; multiple) printf '%s\trefs/heads/main\n%s\trefs/heads/main\n' "$PIHUB_TEST_REMOTE" "$PIHUB_TEST_HEAD";; failure|auth|timeout) exit 1;; esac;;
  *'fetch --no-tags --no-write-fetch-head origin '*) [ "${PIHUB_TEST_FETCH_FAIL:-0}" = 0 ];;
  *'rev-parse --verify '*^{commit}*) printf '%s\n' "$PIHUB_TEST_REMOTE";;
  *'merge-base --is-ancestor'*) [ "${PIHUB_TEST_DIVERGED:-0}" = 0 ];;
  *'rev-list --count'*) printf '%s\n' "${PIHUB_TEST_COUNT:-1}";;
  *'switch --detach'*) :;;
  *) exit 90;;
esac
EOF
cat >"$TMP/bin/docker" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >> "$PIHUB_TEST_DOCKER_LOG"
EOF
chmod 755 "$TMP/bin/git" "$TMP/bin/docker"
sed -e "s|^G=/usr/bin/git$|G=$TMP/bin/git|" -e "s|^D=/usr/bin/docker$|D=$TMP/bin/docker|" -e "s|^R=.*$|R=$TMP/repo|" -e "s|^B=.*$|B=$TMP/repo/docker-compose.yml|" "$ACTION" >"$TMP/action"
chmod 755 "$TMP/action"
run(){ PIHUB_TEST_LOG="$LOG" PIHUB_TEST_DOCKER_LOG="$TMP/docker.log" PIHUB_TEST_HEAD="$HEAD" PIHUB_TEST_REMOTE="$REMOTE" PIHUB_TEST_REMOTE_MODE="${PIHUB_TEST_REMOTE_MODE:-one}" PIHUB_TEST_DIVERGED="${PIHUB_TEST_DIVERGED:-0}" PIHUB_TEST_COUNT="${PIHUB_TEST_COUNT:-1}" PIHUB_TEST_FETCH_FAIL="${PIHUB_TEST_FETCH_FAIL:-0}" "$TMP/action" "$@"; }
expect(){ [ "$1" = "$2" ] || { echo "expected: $1\nactual: $2" >&2; exit 1; }; }
reset(){ : >"$LOG"; : >"$TMP/docker.log"; unset PIHUB_TEST_REMOTE_MODE PIHUB_TEST_DIVERGED PIHUB_TEST_COUNT PIHUB_TEST_FETCH_FAIL; }

# Stale local origin/main is intentionally never exposed to the action.  The
# fixed git log proves PREPARE neither reads it nor updates it/current/worktree.
reset; REMOTE=$HEAD; out=$(run prepare); expect "{\"protocolVersion\":1,\"status\":\"upToDate\",\"currentRevision\":\"$HEAD\",\"targetRevision\":\"$HEAD\",\"changeCount\":0}" "$out"; ! grep -q 'fetch\|refs/remotes/origin/main\|switch' "$LOG"
reset; REMOTE=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb; out=$(run prepare); expect "{\"protocolVersion\":1,\"status\":\"ready\",\"currentRevision\":\"$HEAD\",\"targetRevision\":\"$REMOTE\",\"changeCount\":1}" "$out"; grep -Fx -- "-C $TMP/repo fetch --no-tags --no-write-fetch-head origin $REMOTE" "$LOG" >/dev/null; ! grep -q 'refs/remotes/origin/main\|switch' "$LOG"
reset; REMOTE=cccccccccccccccccccccccccccccccccccccccc; out=$(run prepare); case "$out" in *'"status":"ready"'*'cccccccccccccccccccccccccccccccccccccccc'*) :;; *) exit 1;; esac
for mode in malformed missing multiple failure auth timeout; do reset; PIHUB_TEST_REMOTE_MODE=$mode; out=$(run prepare); expect '{"protocolVersion":1,"status":"blocked","reason":"repositoryUnavailable"}' "$out"; done
reset; PIHUB_TEST_FETCH_FAIL=1; out=$(run prepare); expect '{"protocolVersion":1,"status":"blocked","reason":"repositoryUnavailable"}' "$out"
reset; PIHUB_TEST_DIVERGED=1; out=$(run prepare); expect '{"protocolVersion":1,"status":"blocked","reason":"divergedHistory"}' "$out"
reset; PIHUB_TEST_COUNT=10001; out=$(run prepare); expect '{"protocolVersion":1,"status":"blocked","reason":"changeCountExceeded"}' "$out"
reset; prepared=$REMOTE; REMOTE=dddddddddddddddddddddddddddddddddddddddd; if run apply "$prepared"; then exit 1; fi; ! grep -q 'switch' "$LOG"; [ ! -s "$TMP/docker.log" ]
reset; prepared=$REMOTE; run apply "$prepared" >/dev/null; grep -q 'switch --detach' "$LOG"; grep -q '^build ' "$TMP/docker.log"; grep -q '^compose ' "$TMP/docker.log"
echo 'personal-finance action tests: passed'
