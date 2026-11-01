//! Le fixture vengono costruite solo nella directory temporanea del test.
use pc_governor::MemoryGovernor;
use pc_ingest::{metadata, thumb};

#[test]
fn raf_container_extracts_preview_and_rejects_out_of_bounds_offsets() {
    let tmp = tempfile::tempdir().unwrap();
    let mut jpeg = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(32, 16, image::Rgb([30, 50, 80]))
        .write_to(&mut jpeg, image::ImageFormat::Jpeg)
        .unwrap();
    let jpeg = jpeg.into_inner();
    let mut bytes = vec![0u8; 148];
    bytes[..16].copy_from_slice(b"FUJIFILMCCD-RAW ");
    bytes[84..88].copy_from_slice(&148u32.to_be_bytes());
    bytes[88..92].copy_from_slice(&(jpeg.len() as u32).to_be_bytes());
    bytes[92..96].copy_from_slice(&(148u32 + jpeg.len() as u32).to_be_bytes());
    bytes[96..100].copy_from_slice(&12u32.to_be_bytes());
    bytes.extend_from_slice(&jpeg);
    bytes.extend_from_slice(&[0, 0, 0, 1, 1, 0x11, 0, 4, 0, 64, 0, 128]);
    let file = tmp.path().join("test.RAF");
    std::fs::write(&file, &bytes).unwrap();
    assert_eq!(thumb::embedded_preview(&file).unwrap(), jpeg);
    assert!(metadata::read_exif(&file).unwrap().is_none());
    assert_eq!(pc_ingest::raf::dimensions(&file), Some((128, 64)));
    let decoded = thumb::decode_reduced(&file, "raf", 512, &MemoryGovernor::new(128)).unwrap();
    assert_eq!(decoded.source_size, (32, 16));
    bytes[84..88].copy_from_slice(&u32::MAX.to_be_bytes());
    std::fs::write(&file, &bytes).unwrap();
    assert!(thumb::embedded_preview(&file).is_none());
}

#[test]
fn raf_full_decode_uses_the_embedded_jpeg() {
    let tmp = tempfile::tempdir().unwrap();
    let mut jpeg = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(96, 64, image::Rgb([30, 50, 80]))
        .write_to(&mut jpeg, image::ImageFormat::Jpeg)
        .unwrap();
    let jpeg = jpeg.into_inner();
    let mut bytes = vec![0u8; 148];
    bytes[..16].copy_from_slice(b"FUJIFILMCCD-RAW ");
    bytes[84..88].copy_from_slice(&148u32.to_be_bytes());
    bytes[88..92].copy_from_slice(&(jpeg.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&jpeg);
    let file = tmp.path().join("DSCF0001.RAF");
    std::fs::write(&file, &bytes).unwrap();
    let f = pc_ingest::full::decode_full(&file, "raf", &MemoryGovernor::new(64)).unwrap();
    assert_eq!(f.dimensions(), (96, 64));
}
