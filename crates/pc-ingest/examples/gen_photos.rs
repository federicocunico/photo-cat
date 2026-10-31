//! Genera una cartella di foto sintetiche per prove manuali di import.
//! Uso: `cargo run --release -p pc-ingest --example gen_photos -- <cartella> <numero>`

use exif::experimental::Writer;
use exif::{Field, In, Tag, Value};
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("cartella di destinazione"));
    let n: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(1000);
    let places = ["Toscana", "Dolomiti", "Roma", "Lago di Garda", "Milano"];
    for i in 0..n {
        let year = 2019 + i % 5;
        let place = places[(i / 7) % places.len()];
        let dir = root.join(year.to_string()).join(place);
        std::fs::create_dir_all(&dir).unwrap();
        let (w, h) = if i % 4 == 0 {
            (1200, 1600)
        } else {
            (1600, 1200)
        };
        let hue = (i * 37 % 255) as u8;
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([hue, (x * 255 / w) as u8, (y * 255 / h) as u8 ^ hue])
        });
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80)
            .encode_image(&img)
            .unwrap();
        let date = format!(
            "{year}:{:02}:{:02} {:02}:{:02}:00",
            1 + i % 12,
            1 + i % 28,
            8 + i % 12,
            i % 60
        );
        let fields = [
            Field {
                tag: Tag::Make,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"Sintetica".to_vec()]),
            },
            Field {
                tag: Tag::Model,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"Generatore 1".to_vec()]),
            },
            Field {
                tag: Tag::DateTimeOriginal,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![date.into_bytes()]),
            },
        ];
        let mut wr = Writer::new();
        for f in &fields {
            wr.push_field(f);
        }
        let mut tiff = std::io::Cursor::new(Vec::new());
        wr.write(&mut tiff, false).unwrap();
        let tiff = tiff.into_inner();
        let mut out = jpeg[..2].to_vec();
        out.extend_from_slice(&[0xFF, 0xE1]);
        out.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
        out.extend_from_slice(b"Exif\0\0");
        out.extend_from_slice(&tiff);
        out.extend_from_slice(&jpeg[2..]);
        std::fs::write(dir.join(format!("IMG_{i:04}.JPG")), out).unwrap();
    }
    println!("{n} foto generate in {}", root.display());
}
