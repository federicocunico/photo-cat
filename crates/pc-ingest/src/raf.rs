//! Dimensioni CFA del contenitore RAF, distinte dal JPEG di anteprima.
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
pub fn dimensions(path: &Path) -> Option<(u32, u32)> {
    let mut f = File::open(path).ok()?;
    let mut head = [0; 100];
    f.read_exact(&mut head).ok()?;
    if !head.starts_with(b"FUJIFILMCCD-RAW") {
        return None;
    }
    let off = u32::from_be_bytes(head[92..96].try_into().ok()?) as u64;
    let len = u32::from_be_bytes(head[96..100].try_into().ok()?) as u64;
    let end = off.checked_add(len)?;
    if off < 100 || len < 4 || end > f.metadata().ok()?.len() {
        return None;
    }
    f.seek(SeekFrom::Start(off)).ok()?;
    let mut count = [0; 4];
    f.read_exact(&mut count).ok()?;
    let mut full = None;
    for _ in 0..u32::from_be_bytes(count).min(65536) {
        if f.stream_position().ok()?.checked_add(4)? > end {
            break;
        }
        let mut tag = [0; 4];
        f.read_exact(&mut tag).ok()?;
        let id = u16::from_be_bytes([tag[0], tag[1]]);
        let size = u16::from_be_bytes([tag[2], tag[3]]) as u64;
        let next = f.stream_position().ok()?.checked_add(size)?;
        if next > end {
            return None;
        }
        if size == 4 && (id == 0x100 || id == 0x111) {
            let mut dims = [0; 4];
            f.read_exact(&mut dims).ok()?;
            let h = u16::from_be_bytes([dims[0], dims[1]]) as u32;
            let w = u16::from_be_bytes([dims[2], dims[3]]) as u32;
            if w > 0 && h > 0 {
                if id == 0x111 {
                    return Some((w, h));
                }
                full = Some((w, h));
            }
        }
        f.seek(SeekFrom::Start(next)).ok()?;
    }
    full
}
