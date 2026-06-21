#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for this test script." >&2
  exit 1
fi

HAS_SQLITE3=0
if command -v sqlite3 >/dev/null 2>&1; then
  HAS_SQLITE3=1
elif ! command -v python3 >/dev/null 2>&1; then
  echo "Either sqlite3 or python3 is required for this test script." >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
DB_PATH="$TMP_DIR/slides.db"
IMG_DIR="$TMP_DIR/images"
mkdir -p "$IMG_DIR"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

write_jpeg() {
  local out_path="$1"
  # 1x1 JPEG image, used to create deterministic test fixtures.
  printf '%s' '/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAP//////////////////////////////////////////////////////////////////////////////////////2wBDAf//////////////////////////////////////////////////////////////////////////////////////wAARCAAQABADASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAX/xAAVAQEBAAAAAAAAAAAAAAAAAAABAv/aAAwDAQACEAMQAAAB1A//xAAVEAEBAAAAAAAAAAAAAAAAAAAAEf/aAAgBAQABBQJf/8QAFBEBAAAAAAAAAAAAAAAAAAAAEP/aAAgBAwEBPwF//8QAFBEBAAAAAAAAAAAAAAAAAAAAEP/aAAgBAgEBPwF//8QAFBABAAAAAAAAAAAAAAAAAAAAEP/aAAgBAQAGPwJf/8QAFBABAAAAAAAAAAAAAAAAAAAAEP/aAAgBAQABPyFf/9k=' | base64 -d > "$out_path"
}

count_rows() {
  if [[ "$HAS_SQLITE3" == "1" ]]; then
    sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM images;'
  else
    python3 - "$DB_PATH" <<'PY'
import sqlite3
import sys

con = sqlite3.connect(sys.argv[1])
cur = con.cursor()
print(cur.execute("SELECT COUNT(*) FROM images").fetchone()[0])
con.close()
PY
  fi
}

count_duplicated_paths() {
  if [[ "$HAS_SQLITE3" == "1" ]]; then
    sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM (SELECT Path, COUNT(*) c FROM images GROUP BY Path HAVING c > 1);'
  else
    python3 - "$DB_PATH" <<'PY'
import sqlite3
import sys

con = sqlite3.connect(sys.argv[1])
cur = con.cursor()
query = "SELECT COUNT(*) FROM (SELECT Path, COUNT(*) c FROM images GROUP BY Path HAVING c > 1)"
print(cur.execute(query).fetchone()[0])
con.close()
PY
  fi
}

assert_eq() {
  local expected="$1"
  local actual="$2"
  local message="$3"
  if [[ "$expected" != "$actual" ]]; then
    echo "Assertion failed: $message (expected=$expected actual=$actual)" >&2
    exit 1
  fi
}

cd "$REPO_ROOT"

write_jpeg "$IMG_DIR/one.jpg"
write_jpeg "$IMG_DIR/two.jpeg"

cargo run -- -i \
  --db-path "$DB_PATH" \
  --image-dir "$IMG_DIR" \
  --image-base-dir "$IMG_DIR" \
  --http-prefix "/img" \
  --reset-db >/dev/null

rows_after_install="$(count_rows)"
assert_eq "2" "$rows_after_install" "install should insert two images"

cargo run -- -u \
  --db-path "$DB_PATH" \
  --image-dir "$IMG_DIR" \
  --image-base-dir "$IMG_DIR" \
  --http-prefix "/img" >/dev/null

rows_after_noop_update="$(count_rows)"
assert_eq "2" "$rows_after_noop_update" "update should not duplicate existing images"

write_jpeg "$IMG_DIR/three.JPG"
write_jpeg "$IMG_DIR/four.JPEG"

cargo run -- -u \
  --db-path "$DB_PATH" \
  --image-dir "$IMG_DIR" \
  --image-base-dir "$IMG_DIR" \
  --http-prefix "/img" >/dev/null

rows_after_new_update="$(count_rows)"
assert_eq "4" "$rows_after_new_update" "update should insert only new images"

duplicated_paths="$(count_duplicated_paths)"
assert_eq "0" "$duplicated_paths" "paths should remain unique"

echo "integration_smoke: PASS"