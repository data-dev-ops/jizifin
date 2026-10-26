# Scratch run: check a change against real data without touching it

The live file `./data/finance.duckdb` is locked by the running backend and holds the household's books. Never point an experiment at it. Serve a scratch instance on a private port instead.

Set `S` to your session scratchpad directory (not the repo: copies hold household ciphertext and amounts).

**Credentials:** you never need the passphrase or the household's login proof, and you must not go looking for them (in backups, transcripts, browser storage or the legacy SQLite file). A scratch instance is initialised with a throwaway proof, and a restore keeps its session (`sessions` is not part of a backup), so the throwaway token keeps working on the restored real data. Verified 2026-10-26.

## 1. Start an empty scratch instance

Deployed code (the built image):

```bash
mkdir -p "$S/scratch/data"
docker rm -f jizifin-scratch >/dev/null 2>&1
docker run -d --name jizifin-scratch -p 127.0.0.1:18000:8000 \
  -e JIZIFIN_BIND=0.0.0.0:8000 -e JIZIFIN_DB_PATH=/data/finance.duckdb -e RUST_LOG=info \
  -v "$S/scratch/data:/data" jizifin-backend:latest
```

Uncommitted code (builds from the working tree; the first release build is slow, run it in the background):

```bash
docker run -d --name jizifin-scratch -p 127.0.0.1:18000:8000 \
  -e JIZIFIN_BIND=0.0.0.0:8000 -e JIZIFIN_DB_PATH=/data/finance.duckdb -e RUST_LOG=info \
  -v /opt/jizifin/backend:/src -w /src \
  -v jizifin-cargo-registry:/usr/local/cargo/registry -v jizifin-cargo-target:/target -e CARGO_TARGET_DIR=/target \
  -v "$S/scratch/data:/data" rust:1-bookworm cargo run --release -p jizifin-api
```

Wait for `curl -fs localhost:18000/health` to print `ok` (`docker logs jizifin-scratch` shows build or migration errors). The schema migrates on start.

## 2. Initialise it with a throwaway proof

```bash
proof=$(head -c 24 /dev/urandom | base64 | tr '+/' '-_' | tr -d '=')
curl -s -XPOST localhost:18000/auth/init -H 'content-type: application/json' -d "{\"proof\":\"$proof\"}" \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])' > "$S/scratch/token"
tok=$(cat "$S/scratch/token")
```

An empty instance is enough to check SQL, routes, validation and error codes, and to build hand-made fixtures (`POST /members`, `/categories`, `/entries`; `*_ct` fields take any Base64URL string such as `bWVtYmVyLW9uZQ`).

## 3. Optional: load real data

Ask the user for a backup file: the Settings tab's backup download in the app, saved somewhere they tell you. Then restore it into the scratch instance:

```bash
curl -s -o /dev/null -w '%{http_code}\n' -XPOST localhost:18000/backup -H "authorization: Bearer $tok" --data-binary @/path/to/jizifin-YYYY-MM-DD.duckdb
```

`204` means the copy now holds the household's data and your throwaway token still works. Do not copy the backup into the repo.

## 4. Exercise the change

The scratch server has no `/api` prefix (Caddy strips it on the live host).

```bash
curl -s localhost:18000/settlements/2026-09 -H "authorization: Bearer $tok" | python3 -m json.tool | head -40
curl -s -XPOST localhost:18000/query -H "authorization: Bearer $tok" -H 'content-type: application/json' \
  -d '{"sql":"SELECT kind, COUNT(*) FROM entries GROUP BY kind"}'
```

Report the request and the relevant part of the response as the evidence. Writes are safe here; they only change the copy.

## 5. Clean up

```bash
docker rm -f jizifin-scratch; rm -rf "$S/scratch"
```
