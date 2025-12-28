#!/bin/sh
set -e
cd "$(dirname "$0")"

WORK="${VENDOR_CHECK_DIR:-${TMPDIR:-/tmp}/tcbs-vendor-$$}"
mkdir -p "$WORK"
cleanup() {
  kill "$SERVER_PID" "$FRONT_PID" 2>/dev/null || true
  pg_ctl -D "$WORK/pgdata" stop -m immediate >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

say() { printf '%s\n' "$1"; }

smoke() {
  url="$1"
  for i in $(seq 1 60); do
    curl -s -o /dev/null "$url/api/sections" && break
    sleep 0.5
  done
  code=$(curl -s -o /dev/null -w '%{http_code}' "$url/api/sections")
  [ "$code" = "200" ] || { say "the board did not answer sections: $code"; return 1; }
}

run_vendor() {
  name="$1"
  database_url="$2"
  say "== $name =="
  BIND_ADDR=127.0.0.1:59080 DATABASE_URL="$database_url" \
    ./target/debug/server >"$WORK/$name-server.log" 2>&1 &
  SERVER_PID=$!
  BIND_ADDR=127.0.0.1:59081 SERVER_URL=http://127.0.0.1:59080 \
    ./target/debug/front >"$WORK/$name-front.log" 2>&1 &
  FRONT_PID=$!
  if smoke http://127.0.0.1:59080; then
    front_code=200
    for i in $(seq 1 20); do
      front_code=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:59081/)
      [ "$front_code" = "200" ] && break
      sleep 0.5
    done
    [ "$front_code" = "200" ] && say "$name: ok" || say "$name: front answered $front_code"
  else
    say "$name: failed"
    cat "$WORK/$name-server.log"
  fi
  kill "$SERVER_PID" "$FRONT_PID" 2>/dev/null || true
  wait "$SERVER_PID" "$FRONT_PID" 2>/dev/null || true
}

run_vendor duckdb "duckdb://$WORK/forum.db"

if command -v initdb >/dev/null 2>&1 && command -v pg_ctl >/dev/null 2>&1; then
  initdb -D "$WORK/pgdata" -U postgres --auth=trust >"$WORK/pg-init.log" 2>&1
  pg_ctl -D "$WORK/pgdata" -l "$WORK/pg.log" -o "-p 55432 -k $WORK" start >/dev/null
  psql -h "$WORK" -p 55432 -U postgres -c 'CREATE DATABASE forum;' >/dev/null
  run_vendor postgres "postgres://postgres@127.0.0.1:55432/forum?sslmode=disable"
else
  say "postgres: skipped, no local server available"
fi

if command -v mariadbd >/dev/null 2>&1 || command -v mysqld >/dev/null 2>&1; then
  say "mysql: a server is present but this script does not start one yet"
else
  say "mysql: skipped, no local server available"
fi
