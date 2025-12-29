#!/bin/sh
set -e
cd "$(dirname "$0")"

CHECKOUT="${REFERENCE_CHECKOUT:-../../lorsource}"
[ -d "$CHECKOUT" ] || { echo "no checkout at $CHECKOUT (set REFERENCE_CHECKOUT)"; exit 1; }
CHECKOUT="$(cd "$CHECKOUT" && pwd)"

WORK="${REFERENCE_CHECK_DIR:-${TMPDIR:-/tmp}/tcbs-reference-$$}"
mkdir -p "$WORK"
say() { printf '%s\n' "$1"; }

PG_NAME="tcbs-reference-pg-$$"
SEARCH_NAME="tcbs-reference-search-$$"
LAST_LOGS="${TMPDIR:-/tmp}/tcbs-reference-last"
cleanup() {
  status=$?
  kill "$APP_PID" 2>/dev/null || true
  podman rm -f "$PG_NAME" "$SEARCH_NAME" >/dev/null 2>&1 || true
  rm -rf "$LAST_LOGS"
  mkdir -p "$LAST_LOGS"
  cp "$WORK"/*.log "$LAST_LOGS"/ 2>/dev/null || true
  rm -rf "$WORK"
  [ "$status" = 0 ] || say "failed; logs kept at $LAST_LOGS"
}
trap cleanup EXIT

command -v podman >/dev/null 2>&1 || { say "podman is required"; exit 1; }

SEARCH_IMAGE=$(grep -rho 'opensearchproject/opensearch:[0-9][0-9.]*' "$CHECKOUT/src/test" 2>/dev/null | head -1)
SEARCH_IMAGE="${SEARCH_IMAGE:-opensearchproject/opensearch:3.5.0}"

say "== storage =="
podman run -d --name "$PG_NAME" --rm -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:16 >/dev/null
podman run -d --name "$SEARCH_NAME" --rm \
  -e discovery.type=single-node -e DISABLE_SECURITY_PLUGIN=true -e DISABLE_INSTALL_DEMO_CONFIG=true \
  -e bootstrap.memory_lock=false -e OPENSEARCH_JAVA_OPTS="-Xms512m -Xmx512m" \
  -p 9200:9200 "$SEARCH_IMAGE" >/dev/null

for i in $(seq 1 30); do pg_isready -h 127.0.0.1 -p 5432 -U postgres >/dev/null 2>&1 && break; sleep 1; done
for i in $(seq 1 60); do
  code=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:9200/_cluster/health 2>/dev/null || true)
  [ "$code" = "200" ] && break
  sleep 2
done
curl -s -X PUT http://127.0.0.1:9200/_cluster/settings -H 'Content-Type: application/json' \
  -d '{"transient":{"cluster.routing.allocation.disk.threshold_enabled":false},"persistent":{"cluster.blocks.create_index":null}}' \
  >/dev/null

say "== database =="
export PGHOST=127.0.0.1 PGUSER=postgres PGPASSWORD=postgres
createuser -d -S -R maxcom 2>/dev/null || true
createuser -D -S -R linuxweb 2>/dev/null || true
createuser -D -S -R jamwiki 2>/dev/null || true
psql -c "ALTER USER maxcom WITH PASSWORD 'maxcom'" -U postgres template1 >/dev/null
psql -c "ALTER USER linuxweb WITH PASSWORD 'linuxweb'" -U postgres template1 >/dev/null
psql -c "DROP DATABASE IF EXISTS lor;" -U postgres >/dev/null
PGPASSWORD=maxcom createdb -U maxcom lor
psql -c 'create extension hstore;' -U postgres lor >/dev/null
psql -c 'create extension fuzzystrmatch;' -U postgres lor >/dev/null
PGPASSWORD=maxcom psql -f "$CHECKOUT/sql/demo.db" -U maxcom lor >"$WORK/demo-import.log" 2>&1

say "== config =="
cat > "$CHECKOUT/src/main/webapp/WEB-INF/config.properties" <<'EOF'
MainUrl=http://127.0.0.1:8080/
SecureUrl=http://127.0.0.1:8080/
WSUrl=ws://127.0.0.1:8080/
activemq.path=target/tmp/activemq
HTMLPathPrefix=target/lor-1.0-SNAPSHOT/
upload.path=target/uploads/
Secret=secret
captcha.private=0x0000000000000000000000000000000000000000
captcha.public=10000000-ffff-ffff-ffff-000000000001
EnableHsts=false
admin.emailAddress=specify_your_real_email_if_you_want_receive_messages
comment.isModeratorAllowedToEdit=false
comment.expireMinutesForEdit=30
comment.isEditingAllowedIfAnswersExists=false
comment.scoreValueForEditing=45
Elasticsearch=http://127.0.0.1:9200
jdbc.url=jdbc:postgresql://127.0.0.1:5432/lor?stringtype=unspecified
jdbc.user=linuxweb
jdbc.password=linuxweb
jdbc.poolSize=10
telegram.token=false
fallback.proxy.host=localhost
fallback.proxy.port=3128
cleanOldUserpics=false
cleanOldImages=false
cleanOldBlockedUsers=false
cleanOldDeletedComments=false
EOF

JAVA25=$(find /nix/store -maxdepth 1 -iname '*-openjdk-25.*' -type d 2>/dev/null | grep -v headless | grep -v jre | head -1)
[ -n "$JAVA25" ] || { say "no Java 25 found under /nix/store (this checkout targets it)"; exit 1; }

say "== migrations =="
(cd "$CHECKOUT" && JAVA_HOME="$JAVA25" mvn --batch-mode liquibase:update \
  -Dliquibase.url="jdbc:postgresql://127.0.0.1:5432/lor" \
  -Dliquibase.username=maxcom -Dliquibase.password=maxcom >"$WORK/liquibase.log" 2>&1) \
  || { say "migrations failed, see $WORK/liquibase.log"; exit 1; }

say "== build and run =="
(cd "$CHECKOUT" && JAVA_HOME="$JAVA25" nohup mvn -DskipTests package jetty:run-war \
  >"$WORK/app.log" 2>&1 &)
for i in $(seq 1 180); do
  grep -qE 'Started ServerConnector|BUILD FAILURE' "$WORK/app.log" 2>/dev/null && break
  sleep 2
done

if grep -q 'BUILD FAILURE' "$WORK/app.log" 2>/dev/null; then
  DART=$(find "${TMPDIR:-/tmp}" -maxdepth 4 -path '*dart-sass-maven-plugin*/src/dart' 2>/dev/null | head -1)
  PATCHELF=$(find /nix/store -maxdepth 1 -iname '*-patchelf-*' -type d 2>/dev/null | head -1)
  GLIBC=$(find /nix/store -maxdepth 1 -iname '*-glibc-*' -type d 2>/dev/null | grep -v -- -dev | grep -v bin | head -1)
  if [ -n "$DART" ] && [ -n "$PATCHELF" ] && [ -n "$GLIBC" ] && [ -f "$GLIBC/lib/ld-linux-x86-64.so.2" ]; then
    say "patching the css build's native binary for this system"
    chmod +w "$DART"
    "$PATCHELF/bin/patchelf" --set-interpreter "$GLIBC/lib/ld-linux-x86-64.so.2" --set-rpath "$GLIBC/lib" "$DART"
    chmod 500 "$DART"
    (cd "$CHECKOUT" && JAVA_HOME="$JAVA25" nohup mvn -DskipTests package jetty:run-war \
      >"$WORK/app.log" 2>&1 &)
    for i in $(seq 1 180); do
      grep -qE 'Started ServerConnector|BUILD FAILURE' "$WORK/app.log" 2>/dev/null && break
      sleep 2
    done
  fi
fi

grep -q 'Started ServerConnector' "$WORK/app.log" 2>/dev/null || { say "the app did not start, see $WORK/app.log"; exit 1; }
APP_PID=$(pgrep -f "maven.multiModuleProjectDirectory=$CHECKOUT" | head -1)
APP_PORT=$(grep -oE 'Started ServerConnector.*0\.0\.0\.0:[0-9]+' "$WORK/app.log" | grep -oE '[0-9]+$' | head -1)
APP_PORT="${APP_PORT:-8080}"

say "== search index =="
REF_JAR="$WORK/reference.jar"
REF_LOGIN=$(curl -s -c "$REF_JAR" "http://127.0.0.1:$APP_PORT/login.jsp")
REF_CSRF=$(printf '%s' "$REF_LOGIN" | grep -o 'name="csrf" value="[^"]*"' | head -1 | sed 's/.*value="//;s/"//')
curl -s -o /dev/null -c "$REF_JAR" -b "$REF_JAR" -X POST "http://127.0.0.1:$APP_PORT/login_process" \
  --data-urlencode "nick=maxcom" --data-urlencode "passwd=passwd" --data-urlencode "csrf=$REF_CSRF"
REF_CSRF=$(grep -i csrf_token "$REF_JAR" | awk '{print $7}')
curl -s -o /dev/null -b "$REF_JAR" -X POST "http://127.0.0.1:$APP_PORT/admin/search-reindex" \
  --data-urlencode "action=all" --data-urlencode "csrf=$REF_CSRF"
for i in $(seq 1 30); do
  curl -s "http://127.0.0.1:9200/_cat/indices?v" 2>/dev/null | grep -q messages && break
  sleep 2
done

say "== scenario suite against the reference board =="
TARGET=reference \
REFERENCE_URL="http://127.0.0.1:$APP_PORT" \
REFERENCE_ADMIN=maxcom \
REFERENCE_MODERATOR=Casus \
REFERENCE_READER=ivlad \
REFERENCE_PASSWORD=passwd \
REFERENCE_GROUP=126 \
REFERENCE_GROUP_PATH=/forum/general \
REFERENCE_SECTIONS=/forum/general/ \
REFERENCE_TAG=lortest \
npm test
