# slideshowsetuprust

Scan a directory of JPEG images and populate a SQLite database for slideshow use.

## What It Does

- Install mode (`-i`): full scan and insert behavior.
- Update mode (`-u`): scan for new images and insert only paths not already in the database.
- Supports `.jpg` and `.jpeg` (case-insensitive).
- Stores image orientation and dimensions.

## Build

```bash
cargo build
```

## Usage

The tool requires exactly one mode: `-i` or `-u`.

```bash
cargo run -- \
  -i \
  --db-path /path/to/slides.db \
  --image-dir /path/to/images \
  --image-base-dir /path/to/images \
  --http-prefix /img \
  --reset-db
```

```bash
cargo run -- \
  -u \
  --db-path /path/to/slides.db \
  --image-dir /path/to/images \
  --image-base-dir /path/to/images \
  --http-prefix /img
```

## Arguments

- `-i, --install`: full scan/install behavior.
- `-u, --update`: incremental update behavior.
- `--db-path <DB_PATH>`: path to SQLite database file.
- `--image-dir <IMAGE_DIR>`: directory to scan recursively.
- `--image-base-dir <IMAGE_BASE_DIR>`: path prefix stripped when building HTTP path.
- `--http-prefix <HTTP_PREFIX>`: prefix prepended to stripped image path.
- `--reset-db`: delete existing rows before install (valid only with `-i`).

## Notes

- The database includes a unique index on `Path` to prevent duplicates.
- Update mode appends new rows using the next available `Idx`.

## Smoke Test

Run the integration smoke test:

```bash
./scripts/integration_smoke.sh
```

The script validates:

- install inserts initial rows
- update does not duplicate existing rows
- update inserts newly added images
- duplicate paths do not exist
