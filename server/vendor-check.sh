#!/bin/sh
set -e
cd "$(dirname "$0")"

WORK="${VENDOR_CHECK_DIR:-${TMPDIR:-/tmp}/tcbs-vendor-$$}"
mkdir -p "$WORK"
MARIADB_CONTAINER="tcbs-vendor-check-mariadb-$$"
cleanup() {
  kill "$SERVER_PID" "$FRONT_PID" 2>/dev/null || true
  pg_ctl -D "$WORK/pgdata" stop -m immediate >/dev/null 2>&1 || true
  docker rm -f "$MARIADB_CONTAINER" >/dev/null 2>&1 || true
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
      front_code=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:59081/ || true)
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

run_vendor sqlite "sqlite://$WORK/forum.db"

if command -v initdb >/dev/null 2>&1 && command -v pg_ctl >/dev/null 2>&1; then
  initdb -D "$WORK/pgdata" -U postgres --auth=trust >"$WORK/pg-init.log" 2>&1
  pg_ctl -D "$WORK/pgdata" -l "$WORK/pg.log" -o "-p 55432 -k $WORK" start >/dev/null
  psql -h "$WORK" -p 55432 -U postgres -c 'CREATE DATABASE forum;' >/dev/null
  run_vendor postgres "postgres://postgres@127.0.0.1:55432/forum?sslmode=disable"
else
  say "postgres: skipped, no local server available"
fi

if command -v mariadbd >/dev/null 2>&1 || command -v mysqld >/dev/null 2>&1; then
  run_vendor mysql "mysql://root:forum@127.0.0.1:55306/forum"
elif command -v docker >/dev/null 2>&1; then
  docker run -d --name "$MARIADB_CONTAINER" --rm \
    -e MARIADB_ROOT_PASSWORD=forum -e MARIADB_DATABASE=forum \
    -p 55306:3306 mariadb:11 >"$WORK/mysql-container.log" 2>&1
  for i in $(seq 1 40); do
    docker exec "$MARIADB_CONTAINER" mariadb -uroot -pforum -e 'SELECT 1' >/dev/null 2>&1 && break
    sleep 1
  done
  run_vendor mysql "mysql://root:forum@127.0.0.1:55306/forum"
  docker rm -f "$MARIADB_CONTAINER" >/dev/null 2>&1 || true
else
  say "mysql: skipped, no local server or docker available"
fi
