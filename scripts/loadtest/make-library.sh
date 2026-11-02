#!/usr/bin/env bash
# Crea una libreria di prova per il test di carico di photo-cat (equivalente di make-library.ps1).
#
#   scripts/loadtest/make-library.sh [SORGENTE] [DESTINAZIONE] [NUMERO] [VARIANTI]
#   scripts/loadtest/make-library.sh ~/Foto /data/photo-cat-loadtest 100000 2000
#   scripts/loadtest/make-library.sh --clean /data/photo-cat-loadtest ~/Foto
#
# - Collegamenti fisici (ln, stesso file system) ai file immagine della sorgente, ripetuti in
#   copy-NN/ fino a NUMERO file; varianti generate (ritaglio e ricodifica) con ImageMagick
#   se presente.
# - La sorgente è aperta solo in lettura: numero di file e date di modifica vengono confrontati
#   prima e dopo, e lo script fallisce se cambiano.
# - --clean cancella solo una cartella chiamata photo-cat-loadtest, mai dentro la sorgente.
set -euo pipefail

exts='jpg|jpeg|png|tif|tiff|webp|bmp|gif|heic|heif|avif|dng|cr2|cr3|nef|arw|orf|rw2|raf|pef|srw'

full() { realpath -m "$1"; }

assert_outside() { # $1 percorso, $2 cartella protetta
  local p q
  p="$(full "$1")/"
  q="$(full "$2")/"
  if [[ "$p" == "$q"* || "$q" == "$p"* ]]; then
    echo "Rifiutato: '$1' si sovrappone alla cartella protetta '$2'" >&2
    exit 1
  fi
}

fingerprint() { # numero di file e hash di percorso, dimensione e mtime
  local list
  list="$(cd "$1" && find . -type f -printf '%p|%s|%T@\n' | LC_ALL=C sort)"
  printf '%s file, impronta %s\n' "$(printf '%s' "$list" | grep -c '' || true)" \
    "$(printf '%s' "$list" | sha256sum | cut -c1-16)"
}

if [[ "${1:-}" == "--clean" ]]; then
  target="$(full "${2:-$HOME/photo-cat-loadtest}")"
  source="$(full "${3:-$HOME/Foto}")"
  assert_outside "$target" "$source"
  [[ "$(basename "$target")" == "photo-cat-loadtest" ]] || {
    echo "Rifiutato: la pulizia agisce solo su una cartella chiamata photo-cat-loadtest" >&2
    exit 1
  }
  before="$(fingerprint "$source")"
  rm -rf -- "$target"
  after="$(fingerprint "$source")"
  [[ "$before" == "$after" ]] || { echo "La sorgente risulta cambiata" >&2; exit 1; }
  echo "Cancellato $target; sorgente invariata ($after)"
  exit 0
fi

source="$(full "${1:-$HOME/Foto}")"
target="$(full "${2:-$HOME/photo-cat-loadtest}")"
count="${3:-100000}"
variants="${4:-2000}"
assert_outside "$target" "$source"
[[ -d "$source" ]] || { echo "Sorgente non trovata: $source" >&2; exit 1; }

echo "Impronta di $source (solo lettura)..."
before="$(fingerprint "$source")"
echo "  $before"

library="$target/library"
mkdir -p "$library" "$target/data"
mapfile -d '' images < <(cd "$source" && find . -type f -regextype posix-extended -iregex ".*\.($exts)$" -print0 | LC_ALL=C sort -z)
(( ${#images[@]} > 0 )) || { echo "Nessuna immagine in $source" >&2; exit 1; }
echo "  ${#images[@]} immagini supportate"

wanted=$(( count > variants ? count - variants : 0 ))
made=0
copy=0
while (( made < wanted )); do
  dir="$library/$(printf 'copy-%02d' "$copy")"
  for rel in "${images[@]}"; do
    (( made < wanted )) || break
    dest="$dir/${rel#./}"
    if [[ ! -e "$dest" ]]; then
      mkdir -p "$(dirname "$dest")"
      ln -- "$source/${rel#./}" "$dest"
    fi
    made=$((made + 1))
    (( made % 5000 == 0 )) && echo "  collegamenti: $made / $wanted"
  done
  copy=$((copy + 1))
done

if (( variants > 0 )) && command -v convert >/dev/null; then
  mkdir -p "$library/variants"
  done_v=0
  for rel in "${images[@]}"; do
    (( done_v < variants )) || break
    case "${rel,,}" in *.jpg | *.jpeg) ;; *) continue ;; esac
    dest="$library/variants/$(printf 'v%06d.jpg' "$done_v")"
    # Lettura della sorgente, scrittura solo nella cartella di prova.
    convert "$source/${rel#./}" -gravity center -crop 90%x90%+0+0 +repage -quality 80 "$dest" 2>/dev/null \
      && done_v=$((done_v + 1))
  done
elif (( variants > 0 )); then
  echo "ImageMagick (convert) non trovato: nessuna variante generata"
fi

echo "File in $library: $(find "$library" -type f | wc -l)"
after="$(fingerprint "$source")"
[[ "$before" == "$after" ]] || { echo "$source È CAMBIATA: prima $before, dopo $after" >&2; exit 1; }
echo "Sorgente invariata: $after"
echo
echo "Avvio: PHOTO_CAT_DATA_DIR=$target/data photo-cat  (poi aggiungere la sorgente $library)"
