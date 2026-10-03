//! トレイアイコンの描画。デザインでは 32×32 の座標で描いており、ここでも同じ座標を
//! 実際のピクセル数に合わせて拡大縮小して描く。

pub use crate::config::IconStyle;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
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

/// 文字を描くフォント。前にあるものから順に、最初に読めた 1 つを使う。
const LATIN_FONTS: &[&str] = &["segoeuib.ttf", "arialbd.ttf"];
const CJK_FONTS: &[&str] = &["YuGothB.ttc", "meiryob.ttc", "msgothic.ttc"];

/// 1 文字の字形。座標はフォントの単位のままで、y は上向き。
#[derive(Clone)]
struct Glyph {
    /// 空白など、形のない文字では None
    outline: Option<tiny_skia::Path>,
    advance: f32,
    units_per_em: f32,
}

/// Windows に入っている太字フォントから、描く文字の字形だけを取り出して覚えておく。
///
/// フォントを丸ごと展開すると、日本語のフォントでは 200 MB を超える。
/// 描くのは数字とプロファイル名の頭文字くらいなので、初めて描く文字のときだけファイルを読み、読んだら捨てる。
pub struct Fonts {
    dir: PathBuf,
    /// 見つからなかった文字も None として覚え、ファイルを読み直さない
    glyphs: Mutex<HashMap<char, Option<Glyph>>>,
}

impl Fonts {
    pub fn load() -> Fonts {
        let dir = PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into()))
            .join("Fonts");
        let fonts = Fonts {
            dir,
            glyphs: Mutex::new(HashMap::new()),
        };
        // 数字のスタイルで使う文字は、起動時にまとめて取り出す
        for c in "0123456789–".chars() {
            fonts.glyph(c);
        }
        fonts
    }

    fn glyph(&self, c: char) -> Option<Glyph> {
        let mut glyphs = self.glyphs.lock().unwrap_or_else(|e| e.into_inner());
        glyphs
            .entry(c)
            .or_insert_with(|| {
                find_glyph(&self.dir, LATIN_FONTS, c)
                    .or_else(|| find_glyph(&self.dir, CJK_FONTS, c))
            })
            .clone()
    }
}

fn find_glyph(dir: &Path, names: &[&str], c: char) -> Option<Glyph> {
    for name in names {
        let Ok(data) = std::fs::read(dir.join(name)) else {
            continue;
        };
        let Ok(face) = ttf_parser::Face::parse(&data, 0) else {
            continue;
        };
        let id = face.glyph_index(c)?;
        let mut outline = Outline(PathBuilder::new());
        face.outline_glyph(id, &mut outline);
        return Some(Glyph {
            outline: outline.0.finish(),
            advance: face.glyph_hor_advance(id).unwrap_or(0) as f32,
            units_per_em: face.units_per_em() as f32,
        });
    }
    None
}

struct Outline(PathBuilder);

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y);
    }
    fn close(&mut self) {
        self.0.close();
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
    // ベースラインを y = 0、y を下向きにした px 単位の輪郭に並べる
    let mut paths = Vec::new();
    let mut pen = 0.0f32;
    for glyph in s.chars().filter_map(|c| fonts.glyph(c)) {
        let scale = px / glyph.units_per_em;
        if let Some(path) = glyph
            .outline
            .and_then(|p| p.transform(Transform::from_row(scale, 0.0, 0.0, -scale, pen, 0.0)))
        {
            paths.push(path);
        }
        pen += glyph.advance * scale;
    }
    let Some(bounds) = paths.iter().map(|p| p.bounds()).reduce(|a, b| {
        tiny_skia::Rect::from_ltrb(
            a.left().min(b.left()),
            a.top().min(b.top()),
            a.right().max(b.right()),
            a.bottom().max(b.bottom()),
        )
        .unwrap_or(a)
    }) else {
        return;
    };
    let ox = (cx - (bounds.left() + bounds.right()) / 2.0).round();
    let oy = (cy - (bounds.top() + bounds.bottom()) / 2.0).round();
    for path in &paths {
        pixmap.fill_path(
            path,
            &paint(color),
            FillRule::Winding,
            Transform::from_translate(ox, oy),
            None,
        );
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
