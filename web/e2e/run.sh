#!/bin/sh
set -e
cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"

DB_PORT="${DB_PORT:-55432}"
API_PORT="${API_PORT:-58080}"
WEB_PORT="${WEB_PORT:-58081}"
CONTAINER="tcbs-e2e-pg"

cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  [ -n "$WEB_PID" ] && kill "$WEB_PID" 2>/dev/null || true
  pkill -f "target/[d]ebug/server" 2>/dev/null || true
  pkill -f "serve -l $WEB_PORT" 2>/dev/null || true
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "$CONTAINER" \
  -e POSTGRES_PASSWORD=dev -e POSTGRES_DB=tcbs \
  -p "$DB_PORT:5432" postgres:16-alpine >/dev/null
until docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1; do sleep 1; done

VITE_API_BASE_URL="http://127.0.0.1:$API_PORT" npm run build

(cd "$ROOT/server" && \
  DATABASE_URL="postgres://postgres:dev@127.0.0.1:$DB_PORT/tcbs" \
  BIND_ADDR="127.0.0.1:$API_PORT" \
  cargo run -p server) &
SERVER_PID=$!

npx serve -s -l "$WEB_PORT" dist &
WEB_PID=$!

until curl -s -o /dev/null "http://127.0.0.1:$API_PORT/api/sign-in" -X POST -H 'Content-Type: application/json' -d '{}'; do sleep 1; done
until curl -s -o /dev/null "http://127.0.0.1:$WEB_PORT/"; do sleep 1; done

export BASE_URL="http://127.0.0.1:$WEB_PORT"
node e2e/moderation.mjs
node e2e/register-sign-in.mjs
node e2e/profile.mjs
node e2e/topics.mjs
node e2e/tags.mjs
node e2e/comments.mjs
node e2e/markdown.mjs
node e2e/editing.mjs
node e2e/search.mjs
node e2e/notifications.mjs
node e2e/bookmarks.mjs
node e2e/reactions.mjs
node e2e/polls.mjs
node e2e/feeds.mjs
