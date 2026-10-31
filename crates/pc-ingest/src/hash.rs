//! Impronte dei file: rapida (riconoscimento di file spostati) e completa (duplicati).

use pc_core::AppResult;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const EDGE: u64 = 64 * 1024;

/// Dimensione + primi e ultimi 64 KB: economica anche su file grandi.
pub fn quick_hash(path: &Path) -> AppResult<String> {
    let mut f = File::open(path)?;
    let size = f.metadata()?.len();
    let mut h = blake3::Hasher::new();
    h.update(&size.to_le_bytes());
    let mut buf = vec![0u8; EDGE as usize];
    let n = read_up_to(&mut f, &mut buf)?;
    h.update(&buf[..n]);
    if size > EDGE * 2 {
        f.seek(SeekFrom::End(-(EDGE as i64)))?;
        let n = read_up_to(&mut f, &mut buf)?;
        h.update(&buf[..n]);
    } else if size > EDGE {
        let n = read_up_to(&mut f, &mut buf)?;
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_hex()[..32].to_string())
}

/// Impronta dell'intero contenuto, letta a blocchi (memoria costante).
pub fn content_hash(path: &Path) -> AppResult<String> {
    let mut f = File::open(path)?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_hex().to_string())
}

fn read_up_to(f: &mut File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        let n = f.read(&mut buf[total..])?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_same_hashes_different_content_differs() {
        let tmp = tempfile::tempdir().unwrap();
        let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let a = tmp.path().join("a.jpg");
        let b = tmp.path().join("sub_b.jpg");
        std::fs::write(&a, &big).unwrap();
        std::fs::write(&b, &big).unwrap();
        assert_eq!(quick_hash(&a).unwrap(), quick_hash(&b).unwrap());
        assert_eq!(content_hash(&a).unwrap(), content_hash(&b).unwrap());

        // Differenza nel mezzo: l'impronta rapida non la vede, quella completa sì.
        let mut mid = big.clone();
        mid[150_000] ^= 0xFF;
        std::fs::write(&b, &mid).unwrap();
        assert_eq!(quick_hash(&a).unwrap(), quick_hash(&b).unwrap());
        assert_ne!(content_hash(&a).unwrap(), content_hash(&b).unwrap());

        // Differenza in testa: entrambe la vedono.
        let mut head = big;
        head[10] ^= 0xFF;
        std::fs::write(&b, &head).unwrap();
        assert_ne!(quick_hash(&a).unwrap(), quick_hash(&b).unwrap());

        let small = tmp.path().join("s.png");
        std::fs::write(&small, b"tiny").unwrap();
        assert_eq!(quick_hash(&small).unwrap().len(), 32);
    }
}
