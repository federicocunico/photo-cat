#!/usr/bin/env bash
# Compila il pacchetto deb e ne ricava l'archivio tar.gz per le altre distro.
# Risultato in target/release/dist/: .deb, .tar.gz e un .sha256 per ciascuno.
# L'archivio ha la struttura bin/ + lib/photo-cat/ che Tauri cerca accanto all'eseguibile.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="$(node -p "require('./app/src-tauri/tauri.conf.json').version")"
deb_name="photo-cat_${version}_amd64"
archive_name="photo-cat-${version}-linux-x64"
bundle="target/release/bundle/deb"
dist="target/release/dist"

if [[ "${1:-}" != "--no-build" ]]; then
  npm --prefix app/ui run tauri -- build --ci --bundles deb
fi

staging="$bundle/$deb_name/data/usr"
[[ -x "$staging/bin/photo-cat" && -d "$staging/lib/photo-cat/runtime" ]] || {
  echo "staging del deb incompleto: $staging" >&2
  exit 1
}

rm -rf "$dist"
mkdir -p "$dist/$archive_name"
cp "$bundle/$deb_name.deb" "$dist/"
cp -a "$staging/bin" "$staging/lib" "$dist/$archive_name/"
tar -C "$dist" -czf "$dist/$archive_name.tar.gz" "$archive_name"
rm -rf "${dist:?}/$archive_name"

# Fine riga LF e solo il nome del file: verificabile con `sha256sum -c` dalla cartella dist.
(
  cd "$dist"
  for f in "$deb_name.deb" "$archive_name.tar.gz"; do
    sha256sum "$f" > "$f.sha256"
  done
)
ls -l "$dist"
