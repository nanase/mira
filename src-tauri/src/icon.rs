//! トレイアイコンの描画。デザインでは 32×32 の座標で描いており、ここでも同じ座標を
//! 実際のピクセル数に合わせて拡大縮小して描く。

pub use crate::config::IconStyle;
use tiny_skia::{Color, FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Clone, Copy)]
struct Rgba(u8, u8, u8, u8);

impl Rgba {
    fn color(self) -> Color {
        Color::from_rgba8(self.0, self.1, self.2, self.3)
    }
}

struct Palette {
    levels: [Rgba; 3],
    /// ペース 6 段階の色。モノクロでは使わない
    pace: [Rgba; 6],
    track: Rgba,
    fg: Rgba,
    tick: Rgba,
}

const DARK_PACE: [Rgba; 6] = [
    Rgba(0x3c, 0xc7, 0x5f, 255),
    Rgba(0x40, 0xc8, 0xe0, 255),
    Rgba(0xff, 0xd6, 0x0a, 255),
    Rgba(0xff, 0x9f, 0x0a, 255),
    Rgba(0xff, 0x5a, 0x4f, 255),
    Rgba(0xc7, 0x7d, 0xff, 255),
];

const LIGHT_PACE: [Rgba; 6] = [
    Rgba(0x1b, 0x7a, 0x3a, 255),
    Rgba(0x00, 0x7c, 0x89, 255),
    Rgba(0x8a, 0x6d, 0x00, 255),
    Rgba(0xb2, 0x50, 0x00, 255),
    Rgba(0xc4, 0x00, 0x1a, 255),
    Rgba(0x8a, 0x3f, 0xc2, 255),
];

fn palette(dark_taskbar: bool, mono: bool) -> Palette {
    match (dark_taskbar, mono) {
        (true, false) => Palette {
            levels: [DARK_PACE[0], DARK_PACE[3], DARK_PACE[4]],
            pace: DARK_PACE,
            track: Rgba(255, 255, 255, 56),
            fg: Rgba(255, 255, 255, 255),
            tick: Rgba(255, 255, 255, 255),
        },
        (false, false) => Palette {
            levels: [LIGHT_PACE[0], LIGHT_PACE[3], LIGHT_PACE[4]],
            pace: LIGHT_PACE,
            track: Rgba(0, 0, 0, 41),
            fg: Rgba(0x1a, 0x1a, 0x1a, 255),
            tick: Rgba(0x1a, 0x1a, 0x1a, 255),
        },
        (true, true) => Palette {
            levels: [Rgba(255, 255, 255, 255); 3],
            pace: DARK_PACE,
            track: Rgba(255, 255, 255, 71),
            fg: Rgba(255, 255, 255, 255),
            tick: Rgba(0x8c, 0x8c, 0x8c, 255),
        },
        (false, true) => Palette {
            levels: [Rgba(0x1a, 0x1a, 0x1a, 255); 3],
            pace: LIGHT_PACE,
            track: Rgba(0, 0, 0, 51),
            fg: Rgba(0x1a, 0x1a, 0x1a, 255),
            tick: Rgba(0x76, 0x76, 0x76, 255),
        },
    }
}

/// 1 本の制限の値。`percent` はバーに描く割合（使用済みか残りかは呼び出し側が決める）。
#[derive(Clone, Copy)]
pub struct Meter {
    pub percent: f64,
    pub level: u8,
    /// 時間マーカーの位置（0〜1）。None なら描かない。
    pub marker: Option<f64>,
    /// ペース 6 段階（0〜5）
    pub pace: Option<u8>,
}

pub struct IconSpec {
    pub style: IconStyle,
    pub size: u32,
    pub dark_taskbar: bool,
    pub mono: bool,
    pub session: Option<Meter>,
    pub weekly: Option<Meter>,
    /// 時間マーカーを描くか。数字のスタイルでは描かない。
    pub show_marker: bool,
    /// 時間マーカーをペースの色で描くか。偽なら白（ライトでは黒）。
    pub pace_marker: bool,
    /// プロファイルの頭文字。リングのスタイルだけ中央に描く。
    pub letter: Option<char>,
    /// 表示用の数値（数字スタイル）。
    pub number: Option<u32>,
}

pub struct Fonts {
    latin: Option<fontdue::Font>,
    cjk: Option<fontdue::Font>,
}

impl Fonts {
    /// Windows に入っている太字フォントを使う。見つからなければ文字は描かない。
    pub fn load() -> Fonts {
        let dir = std::path::PathBuf::from(
            std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into()),
        )
        .join("Fonts");
        let load = |names: &[&str]| {
            names.iter().find_map(|name| {
                let bytes = std::fs::read(dir.join(name)).ok()?;
                fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
            })
        };
        Fonts {
            latin: load(&["segoeuib.ttf", "arialbd.ttf"]),
            cjk: load(&["YuGothB.ttc", "meiryob.ttc", "msgothic.ttc"]),
        }
    }

    fn for_char(&self, c: char) -> Option<&fontdue::Font> {
        let has = |f: &&fontdue::Font| f.lookup_glyph_index(c) != 0;
        self.latin
            .as_ref()
            .filter(has)
            .or(self.cjk.as_ref().filter(has))
    }
}

/// 描いたアイコンを RGBA（アルファ乗算なし）で返す。
pub fn render(spec: &IconSpec, fonts: &Fonts) -> Vec<u8> {
    let size = spec.size.max(8);
    let mut pixmap = Pixmap::new(size, size).expect("アイコンのサイズが不正です");
    let s = size as f32 / 32.0;
    let pal = palette(spec.dark_taskbar, spec.mono);
    let level_color = |m: &Meter| pal.levels[m.level.min(2) as usize];
    // 時間マーカーの位置と色。表示しない設定なら None
    let marker = |m: Option<&Meter>| -> Option<(f64, Rgba)> {
        let m = m?;
        let t = m.marker.filter(|_| spec.show_marker)?;
        let color = match m.pace {
            Some(p) if spec.pace_marker && !spec.mono => pal.pace[p.min(5) as usize],
            _ => pal.tick,
        };
        Some((t, color))
    };

    match spec.style {
        IconStyle::Ring | IconStyle::RingTick => {
            ring(
                &mut pixmap,
                s,
                12.0,
                5.0,
                pal.track,
                spec.session.as_ref().map(|m| (m.percent, level_color(m))),
            );
            if let Some((t, color)) = marker(spec.session.as_ref()) {
                radial_tick(&mut pixmap, s, t, 7.5, 15.5, 2.4, color);
            }
            if let Some(c) = spec.letter {
                text(
                    &mut pixmap,
                    fonts,
                    &c.to_string(),
                    11.0 * s,
                    16.0 * s,
                    16.0 * s,
                    pal.fg,
                );
            }
        }
        IconStyle::Double => {
            ring(
                &mut pixmap,
                s,
                13.5,
                3.5,
                pal.track,
                spec.session.as_ref().map(|m| (m.percent, level_color(m))),
            );
            ring(
                &mut pixmap,
                s,
                7.5,
                3.5,
                pal.track,
                spec.weekly.as_ref().map(|m| (m.percent, level_color(m))),
            );
            // 目盛りはそれぞれのリングの幅だけを横切る
            if let Some((t, color)) = marker(spec.session.as_ref()) {
                radial_tick(&mut pixmap, s, t, 11.0, 16.0, 2.0, color);
            }
            if let Some((t, color)) = marker(spec.weekly.as_ref()) {
                radial_tick(&mut pixmap, s, t, 5.0, 10.0, 2.0, color);
            }
        }
        IconStyle::Number => match (spec.number, spec.session.as_ref()) {
            (Some(n), Some(m)) => {
                let px = if n >= 100 { 14.0 } else { 19.0 };
                text(
                    &mut pixmap,
                    fonts,
                    &n.to_string(),
                    px * s,
                    16.0 * s,
                    16.0 * s,
                    level_color(m),
                );
            }
            _ => text(
                &mut pixmap,
                fonts,
                "–",
                19.0 * s,
                16.0 * s,
                16.0 * s,
                pal.track,
            ),
        },
        IconStyle::Bars => {
            bar(
                &mut pixmap,
                s,
                5.0,
                pal.track,
                spec.session.as_ref().map(|m| (m.percent, level_color(m))),
            );
            bar(
                &mut pixmap,
                s,
                18.0,
                pal.track,
                spec.weekly.as_ref().map(|m| (m.percent, level_color(m))),
            );
            for (x, m) in [(5.0, spec.session.as_ref()), (18.0, spec.weekly.as_ref())] {
                if let Some((t, color)) = marker(m) {
                    bar_tick(&mut pixmap, s, x, t, color);
                }
            }
        }
    }

    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

fn paint(color: Rgba) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color.color());
    paint.anti_alias = true;
    paint
}

/// 12 時の位置から時計回りに弧を描く。
fn ring(pixmap: &mut Pixmap, s: f32, r: f32, width: f32, track: Rgba, value: Option<(f64, Rgba)>) {
    let stroke = Stroke {
        width: width * s,
        line_cap: LineCap::Butt,
        ..Stroke::default()
    };
    if let Some(path) = PathBuilder::from_circle(16.0 * s, 16.0 * s, r * s) {
        pixmap.stroke_path(&path, &paint(track), &stroke, Transform::identity(), None);
    }
    let Some((percent, color)) = value else {
        return;
    };
    let frac = (percent / 100.0).clamp(0.0, 1.0) as f32;
    if frac <= 0.0 {
        return;
    }
    if frac >= 0.999 {
        if let Some(path) = PathBuilder::from_circle(16.0 * s, 16.0 * s, r * s) {
            pixmap.stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
        }
        return;
    }
    let steps = ((frac * 96.0).ceil() as usize).max(4);
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let a =
            -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac * i as f32 / steps as f32;
        let (x, y) = ((16.0 + r * a.cos()) * s, (16.0 + r * a.sin()) * s);
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
    }
}

fn line(pixmap: &mut Pixmap, s: f32, from: (f32, f32), to: (f32, f32), width: f32, color: Rgba) {
    let mut pb = PathBuilder::new();
    pb.move_to(from.0 * s, from.1 * s);
    pb.line_to(to.0 * s, to.1 * s);
    let stroke = Stroke {
        width: (width * s).max(1.0),
        line_cap: LineCap::Round,
        ..Stroke::default()
    };
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
    }
}

/// リングの経過時間の位置に、半径 r_in から r_out までの目盛りを描く。
fn radial_tick(
    pixmap: &mut Pixmap,
    s: f32,
    fraction: f64,
    r_in: f32,
    r_out: f32,
    width: f32,
    color: Rgba,
) {
    let a = fraction as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let at = |r: f32| (16.0 + r * a.cos(), 16.0 + r * a.sin());
    line(pixmap, s, at(r_in), at(r_out), width, color);
}

/// 縦バーの経過時間の高さに、横向きの目盛りを描く。
fn bar_tick(pixmap: &mut Pixmap, s: f32, x: f32, fraction: f64, color: Rgba) {
    let y = 3.0 + 26.0 * (1.0 - fraction.clamp(0.0, 1.0) as f32);
    line(pixmap, s, (x - 1.0, y), (x + 10.0, y), 2.0, color);
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

/// 下から伸びる縦バー。
fn bar(pixmap: &mut Pixmap, s: f32, x: f32, track: Rgba, value: Option<(f64, Rgba)>) {
    if let Some(path) = rounded_rect(x * s, 3.0 * s, 9.0 * s, 26.0 * s, 2.0 * s) {
        pixmap.fill_path(
            &path,
            &paint(track),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    let Some((percent, color)) = value else {
        return;
    };
    let h = 26.0 * (percent / 100.0).clamp(0.0, 1.0) as f32;
    if h <= 0.0 {
        return;
    }
    if let Some(path) = rounded_rect(x * s, (3.0 + 26.0 - h) * s, 9.0 * s, h * s, 2.0 * s) {
        pixmap.fill_path(
            &path,
            &paint(color),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// 文字列の外接矩形の中心が (cx, cy) に来るように描く。
fn text(pixmap: &mut Pixmap, fonts: &Fonts, s: &str, px: f32, cx: f32, cy: f32, color: Rgba) {
    let mut glyphs = Vec::new();
    let mut pen = 0.0f32;
    for c in s.chars() {
        let Some(font) = fonts.for_char(c) else {
            continue;
        };
        let (metrics, bitmap) = font.rasterize(c, px);
        glyphs.push((pen.round() as i32 + metrics.xmin, metrics, bitmap));
        pen += metrics.advance_width;
    }
    if glyphs.is_empty() {
        return;
    }
    // ベースラインを y = 0 とした外接矩形
    let left = glyphs.iter().map(|(x, _, _)| *x).min().unwrap_or(0);
    let right = glyphs
        .iter()
        .map(|(x, m, _)| x + m.width as i32)
        .max()
        .unwrap_or(0);
    let top = glyphs
        .iter()
        .map(|(_, m, _)| -(m.ymin + m.height as i32))
        .min()
        .unwrap_or(0);
    let bottom = glyphs.iter().map(|(_, m, _)| -m.ymin).max().unwrap_or(0);
    let ox = (cx - (left + right) as f32 / 2.0).round() as i32;
    let oy = (cy - (top + bottom) as f32 / 2.0).round() as i32;

    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    let pixels = pixmap.pixels_mut();
    for (gx, m, bitmap) in &glyphs {
        let gy = -(m.ymin + m.height as i32);
        for row in 0..m.height as i32 {
            for col in 0..m.width as i32 {
                let (x, y) = (ox + gx + col, oy + gy + row);
                if x < 0 || y < 0 || x >= w || y >= h {
                    continue;
                }
                let coverage = bitmap[(row * m.width as i32 + col) as usize] as f32 / 255.0;
                let a = coverage * color.3 as f32 / 255.0;
                if a <= 0.0 {
                    continue;
                }
                let dst = pixels[(y * w + x) as usize];
                let blend = |src: u8, d: u8| {
                    (src as f32 * a + d as f32 * (1.0 - a)).round().min(255.0) as u8
                };
                let out_a = (a * 255.0 + dst.alpha() as f32 * (1.0 - a))
                    .round()
                    .min(255.0) as u8;
                let r = blend(color.0, dst.red()).min(out_a);
                let g = blend(color.1, dst.green()).min(out_a);
                let b = blend(color.2, dst.blue()).min(out_a);
                if let Some(p) = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, out_a) {
                    pixels[(y * w + x) as usize] = p;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_every_style_at_small_sizes() {
        let fonts = Fonts::load();
        for style in [
            IconStyle::Ring,
            IconStyle::RingTick,
            IconStyle::Double,
            IconStyle::Number,
            IconStyle::Bars,
        ] {
            for size in [16, 20, 24, 32] {
                let spec = IconSpec {
                    style,
                    size,
                    dark_taskbar: true,
                    mono: false,
                    session: Some(Meter {
                        percent: 39.0,
                        level: 0,
                        marker: Some(0.57),
                        pace: Some(1),
                    }),
                    weekly: Some(Meter {
                        percent: 62.0,
                        level: 1,
                        marker: Some(0.72),
                        pace: Some(3),
                    }),
                    show_marker: true,
                    pace_marker: size % 16 == 0,
                    letter: Some('M'),
                    number: Some(39),
                };
                let rgba = render(&spec, &fonts);
                assert_eq!(rgba.len(), (size * size * 4) as usize);
                assert!(rgba.chunks(4).any(|p| p[3] > 0), "{style:?} {size} が空");
            }
        }
    }
}
