//! Turns `assets/tray.png` into raw ARGB32 (the format StatusNotifierItem wants)
//! so the daemon needs no image decoder at run time.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("../../assets/tray.png");
    println!("cargo:rerun-if-changed={}", src.display());

    let bytes = std::fs::read(&src).expect("assets/tray.png is missing");
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("tray.png is not a valid PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("tray.png is too large")];
    let info = reader.next_frame(&mut buf).expect("tray.png is corrupt");
    let px = &buf[..info.buffer_size()];

    let mut argb = Vec::with_capacity((info.width * info.height * 4) as usize);
    match info.color_type {
        png::ColorType::Rgba => px
            .as_chunks::<4>()
            .0
            .iter()
            .for_each(|p| argb.extend_from_slice(&[p[3], p[0], p[1], p[2]])),
        png::ColorType::Rgb => px
            .as_chunks::<3>()
            .0
            .iter()
            .for_each(|p| argb.extend_from_slice(&[255, p[0], p[1], p[2]])),
        png::ColorType::GrayscaleAlpha => px
            .as_chunks::<2>()
            .0
            .iter()
            .for_each(|p| argb.extend_from_slice(&[p[1], p[0], p[0], p[0]])),
        png::ColorType::Grayscale => px
            .iter()
            .for_each(|&g| argb.extend_from_slice(&[255, g, g, g])),
        other => panic!("unsupported PNG colour type {other:?}"),
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("tray.argb"), argb).unwrap();
    std::fs::write(
        out.join("tray_icon.rs"),
        format!(
            "pub const WIDTH: i32 = {};\npub const HEIGHT: i32 = {};\n\
             pub static ARGB: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/tray.argb\"));\n",
            info.width, info.height
        ),
    )
    .unwrap();
}
