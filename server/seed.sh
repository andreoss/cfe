#!/bin/sh
set -e

API="${API:-http://127.0.0.1:58080}"
PASS="${SEED_PASSWORD:-correcthorse}"
JAR_DIR="${SEED_JAR_DIR:-${TMPDIR:-/tmp}/tcbs-seed-$$}"
mkdir -p "$JAR_DIR"
trap 'rm -rf "$JAR_DIR"' EXIT

PNG='iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGP4DwABAQEAGdiK2wAAAABJRU5ErkJggg=='

stamp=$(date +%s)
say() { printf '%s\n' "$1"; }

jar() { printf '%s/%s.txt' "$JAR_DIR" "$1"; }

api() {
  method="$1"
  path="$2"
  who="$3"
  data="$4"
  if [ -n "$who" ]; then
    set -- -b "$(jar "$who")" -c "$(jar "$who")"
  else
    set --
  fi
  if [ -n "$data" ]; then
    curl -s -X "$method" "$API$path" -H 'Content-Type: application/json' -d "$data" "$@"
  else
    curl -s -X "$method" "$API$path" "$@"
  fi
}

field() { grep -o "\"$1\":\"[^\"]*\"" | head -1 | cut -d'"' -f4; }

nth_field() { grep -o "\"$1\":\"[^\"]*\"" | sed -n "$2p" | cut -d'"' -f4; }

register() {
  curl -s -o /dev/null -c "$(jar "$1")" -X POST "$API/api/register" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"$1\",\"email\":\"$1@example.com\",\"password\":\"$PASS\"}"
}

sign_in() {
  curl -s -o /dev/null -c "$(jar "$1")" -X POST "$API/api/sign-in" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"$1\",\"password\":\"$PASS\"}"
}

until curl -s -o /dev/null "$API/api/sections"; do sleep 1; done

MOD="seed_mod_$stamp"
ALICE="seed_alice_$stamp"
BOB="seed_bob_$stamp"
CARL="seed_carl_$stamp"
LOUD="seed_loud_$stamp"

say "accounts"
for who in "$MOD" "$ALICE" "$BOB" "$CARL" "$LOUD"; do
  register "$who"
done

say "sessions and remembered addresses"
for who in "$ALICE" "$BOB" "$CARL"; do
  sign_in "$who"
done

say "a refused sign-in"
curl -s -o /dev/null -X POST "$API/api/sign-in" -H 'Content-Type: application/json' \
  -d "{\"username\":\"$LOUD\",\"password\":\"wrong-on-purpose\"}"

say "a moderator"
OPERATOR="${SEED_MOD_USER:-admin}"
OPERATOR_PASS="${SEED_MOD_PASS:-admin}"
curl -s -o /dev/null -c "$(jar "$OPERATOR")" -X POST "$API/api/sign-in" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$OPERATOR\",\"password\":\"$OPERATOR_PASS\"}"
api POST "/api/users/$MOD/promote" "$OPERATOR" >/dev/null
if ! api GET "/api/me" "$MOD" | grep -q '"role":"moderator"'; then
  say "could not sign in as $OPERATOR: set SEED_MOD_USER and SEED_MOD_PASS"
  exit 1
fi
api POST "/api/users/$CARL/role" "$MOD" "{\"role\":\"corrector\"}" >/dev/null

say "profiles and avatars"
api PATCH "/api/me/bio" "$ALICE" '{"bio":"Seeded account."}' >/dev/null
api POST "/api/me/avatar" "$ALICE" "{\"data\":\"$PNG\"}" >/dev/null

say "groups"
api POST "/api/sections/general/groups" "$MOD" \
  "{\"name\":\"Seeded group\",\"slug\":\"seeded-$stamp\"}" >/dev/null

say "topics and tags"
TOPIC=$(api POST "/api/sections/general/topics" "$ALICE" \
  "{\"title\":\"A seeded subject $stamp\",\"body\":\"A seeded body.\",\"tags\":[\"seeded\",\"demo\"]}" \
  | field id)
api POST "/api/topics/$TOPIC/publish" "$MOD" >/dev/null 2>&1 || true
SECOND=$(api POST "/api/sections/help/topics" "$BOB" \
  "{\"title\":\"A second seeded subject $stamp\",\"body\":\"Another body.\",\"tags\":[\"seeded\"]}" \
  | field id)
api POST "/api/topics/$SECOND/publish" "$MOD" >/dev/null 2>&1 || true

say "topic lifecycle"
api POST "/api/topics/$TOPIC/sticky" "$MOD" '{"sticky":true}' >/dev/null
api POST "/api/topics/$TOPIC/resolved" "$MOD" '{"resolved":true}' >/dev/null
api POST "/api/topics/$SECOND/off-front" "$MOD" '{"off_front":true}' >/dev/null
api POST "/api/topics/$TOPIC/postscore" "$MOD" '{"postscore":5}' >/dev/null

say "comments"
COMMENT=$(api POST "/api/topics/$TOPIC/comments" "$BOB" \
  '{"body":"A seeded comment."}' | field id)
api POST "/api/topics/$TOPIC/comments" "$CARL" \
  "{\"body\":\"A seeded reply.\",\"parent_id\":\"$COMMENT\"}" >/dev/null

say "reactions"
api POST "/api/topics/$TOPIC/reactions" "$BOB" '{"kind":"like"}' >/dev/null
api POST "/api/topics/$TOPIC/comments/$COMMENT/reactions" "$ALICE" '{"kind":"like"}' >/dev/null

say "a poll and a vote"
api POST "/api/topics/$TOPIC/poll" "$ALICE" \
  '{"question":"Which one?","options":["The first","The second"]}' >/dev/null
OPTION=$(api GET "/api/topics/$TOPIC/poll" "$BOB" | nth_field id 2)
api POST "/api/topics/$TOPIC/poll/vote" "$BOB" "{\"option_id\":\"$OPTION\"}" >/dev/null

say "saved and watched topics"
api POST "/api/topics/$TOPIC/bookmark" "$BOB" >/dev/null
api POST "/api/topics/$SECOND/watch" "$ALICE" >/dev/null

say "a private note"
api PUT "/api/users/$BOB/remark" "$ALICE" '{"text":"A seeded note."}' >/dev/null

say "an invitation, issued and spent"
CODE=$(api POST "/api/invitations" "$ALICE" | field code)
SPARE=$(api POST "/api/invitations" "$ALICE" | field code)
curl -s -o /dev/null -X POST "$API/api/register" -H 'Content-Type: application/json' \
  -d "{\"username\":\"seed_invited_$stamp\",\"email\":\"seed_invited_$stamp@example.com\",\"password\":\"$PASS\",\"invitation\":\"$CODE\"}"
say "one code left unused: $SPARE"

say "a report, and one closed"
REPORT=$(api POST "/api/topics/$TOPIC/report" "$BOB" \
  '{"kind":"rule","reason":"A seeded report."}' | field id)
api POST "/api/topics/$TOPIC/comments/$COMMENT/report" "$CARL" \
  '{"kind":"spelling","reason":"A second seeded report."}' >/dev/null
api POST "/api/reports/$REPORT/close" "$MOD" >/dev/null

say "moderation against one account"
api POST "/api/users/$LOUD/warn" "$MOD" '{"reason":"A seeded warning."}' >/dev/null
api POST "/api/users/$LOUD/ban" "$MOD" '{"reason":"A seeded ban.","days":7}' >/dev/null
api POST "/api/users/$LOUD/ignore" "$ALICE" >/dev/null

say "a blocked address"
api POST "/api/address-blocks" "$MOD" \
  '{"addr":"203.0.113.7","reason":"A seeded block.","days":30}' >/dev/null

say "a password reset, for a mail token"
curl -s -o /dev/null -X POST "$API/api/password-reset" -H 'Content-Type: application/json' \
  -d "{\"email\":\"$ALICE@example.com\"}"
api POST "/api/me/email" "$BOB" "{\"email\":\"seed_bob_new_$stamp@example.com\"}" >/dev/null

say "seeded"
