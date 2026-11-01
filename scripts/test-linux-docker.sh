#!/usr/bin/env bash
# Test di build Linux in Docker, per chi sviluppa su Windows o macOS (make test-linux).
# Ripete i passi del job "linux" della CI (Ubuntu 22.04): build UI, fmt, clippy,
# test Rust e pacchetti .deb/.tar.gz.
#
# Usa lo stato corrente del repository: commit più modifiche ai file tracciati.
# I file nuovi devono essere almeno in stage (git add), altrimenti non vengono copiati.
# I sorgenti escono da git con fine riga LF, come su un checkout Linux.
#
#   scripts/test-linux-docker.sh                 tutti i passi
#   SKIP_PACKAGE=1 scripts/test-linux-docker.sh  senza pacchetti
#
# Cache in volumi Docker: pc-linux-target, pc-linux-nm, pc-linux-cargo
# (si eliminano con: docker volume rm pc-linux-target pc-linux-nm pc-linux-cargo).
set -euo pipefail
export MSYS_NO_PATHCONV=1 # Git Bash su Windows: non convertire i percorsi del container

cd "$(git rev-parse --show-toplevel)"
image=photo-cat-linux-ci
docker build -q -t "$image" scripts/linux-docker >/dev/null

ref=$(git stash create)
git -c core.autocrlf=false archive "${ref:-HEAD}" | docker run --rm -i \
  -e SKIP_PACKAGE="${SKIP_PACKAGE:-}" \
  -v pc-linux-target:/src/target \
  -v pc-linux-nm:/src/app/ui/node_modules \
  -v pc-linux-cargo:/root/.cargo/registry \
  "$image" bash -c 'tar -x -C /src && bash scripts/linux-docker/ci.sh'
