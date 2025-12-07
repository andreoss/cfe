#!/bin/sh
set -e
cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"

VENDOR="${VENDOR:-postgres}"
DB_PORT="${DB_PORT:-55432}"
API_PORT="${API_PORT:-58080}"
WEB_PORT="${WEB_PORT:-58081}"
CONTAINER="tcbs-e2e-$VENDOR"
DB_FILE="${DB_FILE:-/tmp/tcbs-e2e-duck.db}"
MAIL_LOG="${MAIL_LOG:-/tmp/tcbs-e2e-mail.log}"
rm -f "$MAIL_LOG"

cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  [ -n "$WEB_PID" ] && kill "$WEB_PID" 2>/dev/null || true
  pkill -f "target/[d]ebug/server" 2>/dev/null || true
  pkill -f "serve -s -l $WEB_PORT" 2>/dev/null || true
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  find "${TMPDIR:-/tmp}" -maxdepth 1 -name 'org.chromium.Chromium.*' \
    -newer "$STARTED_AT" -exec rm -rf {} + 2>/dev/null || true
  rm -f "$STARTED_AT"
}
STARTED_AT=$(mktemp)
trap cleanup EXIT
docker rm -f "$CONTAINER" >/dev/null 2>&1 || true

case "$VENDOR" in
  postgres)
    docker run -d --name "$CONTAINER" \
      -e POSTGRES_PASSWORD=dev -e POSTGRES_DB=tcbs \
      -p "$DB_PORT:5432" postgres:16-alpine >/dev/null
    until docker exec "$CONTAINER" psql -U postgres -d tcbs -c 'select 1' >/dev/null 2>&1; do
      sleep 1
    done
    DATABASE_URL="postgres://postgres:dev@127.0.0.1:$DB_PORT/tcbs"
    ;;
  mysql)
    docker run -d --name "$CONTAINER" \
      -e MYSQL_ROOT_PASSWORD=dev -e MYSQL_DATABASE=tcbs \
      -p "$DB_PORT:3306" mysql:8.0-debian >/dev/null
    until docker exec "$CONTAINER" \
      mysql -h 127.0.0.1 --protocol=TCP -uroot -pdev -e 'select 1' tcbs >/dev/null 2>&1; do
      sleep 2
    done
    DATABASE_URL="mysql://root:dev@127.0.0.1:$DB_PORT/tcbs"
    ;;
  duckdb)
    rm -f "$DB_FILE"
    DATABASE_URL="duckdb://$DB_FILE"
    ;;
  *)
    echo "unsupported vendor: $VENDOR" >&2
    exit 1
    ;;
esac

VITE_API_BASE_URL="http://127.0.0.1:$API_PORT" npm run build

start_api() {
  (cd "$ROOT/server" && \
    DATABASE_URL="$DATABASE_URL" \
    BIND_ADDR="127.0.0.1:$API_PORT" \
    MAIL_TRANSPORT=log \
    MAIL_LOG="$MAIL_LOG" \
    RATE_LIMIT_MAX="${1:-100000}" \
    SIGN_IN_ATTEMPT_MAX="${SIGN_IN_ATTEMPT_MAX:-100000}" \
    CHALLENGE_TRANSPORT="${CHALLENGE_TRANSPORT:-off}" \
    CHALLENGE_SECRET="${CHALLENGE_SECRET:-}" \
    CHALLENGE_ON_REGISTER="${CHALLENGE_ON_REGISTER:-0}" \
    CHALLENGE_BELOW_FLOOR="${CHALLENGE_BELOW_FLOOR:-0}" \
    ACCOUNT_RATE_LIMIT_MAX="${ACCOUNT_RATE_LIMIT_MAX:-100000}" \
    SLOW_MODE_SCORE_FLOOR="${2:--1000}" \
    SLOW_MODE_INTERVAL_SECONDS="${3:-120}" \
    RATE_LIMIT_WINDOW_SECONDS="${4:-60}" \
    MAINTENANCE_INTERVAL_SECONDS=0 \
    MAINTENANCE_SCORE_FLOOR="${MAINTENANCE_SCORE_FLOOR:--50}" \
    CONFIRMATION_WINDOW_SECONDS="${CONFIRMATION_WINDOW_SECONDS:-604800}" \
    INVITATION_REQUIRED="${INVITATION_REQUIRED:-0}" \
    INVITATION_MAX_OUTSTANDING="${INVITATION_MAX_OUTSTANDING:-5}" \
    cargo run -p server) &
  SERVER_PID=$!
  until curl -s -o /dev/null "http://127.0.0.1:$API_PORT/api/sign-in" \
    -X POST -H 'Content-Type: application/json' -d '{}'; do sleep 1; done
}

restart_api() {
  kill "$SERVER_PID" 2>/dev/null || true
  pkill -f "target/[d]ebug/server" 2>/dev/null || true
  while curl -s -o /dev/null "http://127.0.0.1:$API_PORT/api/sections"; do sleep 1; done
  start_api "$@"
}

start_api "${RATE_LIMIT_MAX:-100000}" "${SLOW_MODE_SCORE_FLOOR:--1000}"

npx serve -s -l "$WEB_PORT" dist &
WEB_PID=$!

waited=0
until curl -s -o /dev/null "http://127.0.0.1:$WEB_PORT/"; do
  waited=$((waited + 1))
  if [ "$waited" -gt 120 ]; then
    echo "the static server did not take port $WEB_PORT; set WEB_PORT to a free one" >&2
    exit 1
  fi
  sleep 1
done

export BASE_URL="http://127.0.0.1:$WEB_PORT"
export API_URL="http://127.0.0.1:$API_PORT"
export MAIL_LOG
export E2E_ROOT_USER="${OPERATOR_USERNAME:-admin}"
export E2E_ROOT_PASS="${OPERATOR_PASSWORD:-admin}"
if [ -n "$SPEC" ]; then
  case "$SPEC" in
    abuse-slow) restart_api 100000 1000 3600 ;;
    abuse-rate) ACCOUNT_RATE_LIMIT_MAX=2 restart_api 100 -1000 120 5 ;;
    maintenance) CONFIRMATION_WINDOW_SECONDS=1 restart_api ;;
    sessions) SIGN_IN_ATTEMPT_MAX=3 restart_api ;;
    invitations-only) INVITATION_REQUIRED=1 restart_api ;;
    challenge)
      CHALLENGE_TRANSPORT=secret CHALLENGE_SECRET="open sesame" \
        CHALLENGE_ON_REGISTER=1 CHALLENGE_BELOW_FLOOR=1 restart_api 100000 5 1
      ;;
  esac
  node "e2e/$SPEC.mjs"
  exit 0
fi

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
node e2e/avatars.mjs
node e2e/account.mjs
node e2e/enforcement.mjs
node e2e/activity.mjs
node e2e/paging.mjs
node e2e/recovery.mjs
node e2e/reputation.mjs
node e2e/groups.mjs
node e2e/permissions.mjs
node e2e/reports.mjs
node e2e/lifecycle.mjs
node e2e/corrector.mjs
node e2e/watching.mjs
node e2e/notes.mjs
node e2e/invitations.mjs
node e2e/section-settings.mjs
node e2e/archive.mjs
node e2e/penalty.mjs
node e2e/restore.mjs
node e2e/history.mjs
node e2e/thread.mjs
node e2e/mentions.mjs
node e2e/tag-depth.mjs
node e2e/images.mjs
node e2e/live-comments.mjs
node e2e/search-filters.mjs
node e2e/operator.mjs
node e2e/display.mjs

restart_api 100000 1000 3600
node e2e/abuse-slow.mjs

ACCOUNT_RATE_LIMIT_MAX=2 restart_api 100 -1000 120 5
node e2e/abuse-rate.mjs

CONFIRMATION_WINDOW_SECONDS=1 restart_api
node e2e/maintenance.mjs

node e2e/addresses.mjs

SIGN_IN_ATTEMPT_MAX=3 restart_api
node e2e/sessions.mjs

INVITATION_REQUIRED=1 restart_api
node e2e/invitations-only.mjs

CHALLENGE_TRANSPORT=secret CHALLENGE_SECRET="open sesame" \
  CHALLENGE_ON_REGISTER=1 CHALLENGE_BELOW_FLOOR=1 restart_api 100000 5 1
node e2e/challenge.mjs
