//! アプリのアイコンを icons/icon.svg から描き出す。
//! `cargo run --example gen_icons` で作り直せる。

use resvg::{tiny_skia, usvg};
use std::path::Path;

fn render(tree: &usvg::Tree, size: u32) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).unwrap();
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().unwrap()
}

/// PNG を埋め込んだ ICO を組み立てる。
fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("icons");
    let svg = std::fs::read(dir.join("icon.svg")).unwrap();
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default()).unwrap();

    for (name, size) in [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
    ] {
        std::fs::write(dir.join(name), render(&tree, size)).unwrap();
    }
    let images: Vec<(u32, Vec<u8>)> = [16, 20, 24, 32, 48, 64, 128, 256]
        .into_iter()
        .map(|size| (size, render(&tree, size)))
        .collect();
    std::fs::write(dir.join("icon.ico"), ico(&images)).unwrap();
    // 設定画面のサイドバーに出す分
    std::fs::write(root.join("../ui/icon.png"), render(&tree, 128)).unwrap();
    println!("{} にアイコンを書きました", dir.display());
}
