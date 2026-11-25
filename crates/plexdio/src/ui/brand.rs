//! Plexdio and KruxSYS marks drawn as vector shapes, plus the "by KruxSYS" credit.
//!
//! The geometry follows `assets/brand/plexdio.svg` and `assets/brand/kruxsys.svg` (64-unit
//! viewBox), so the marks stay sharp at any size and zoom. Below [`SMALL`] px the Plexdio mark
//! switches to the pixel-aligned small variant (`plexdio-16.svg`): solid glyph, no signal square.
//! See docs/brand.md.

use crate::ui::theme;
use egui::{pos2, vec2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke};

/// Below this size (px) the Plexdio mark uses the small variant.
pub const SMALL: f32 = 28.0;
/// Edge of the small icon variant: a step lighter than `LINE_2` so it reads on dark taskbars.
const SMALL_EDGE: Color32 = Color32::from_rgb(0x3a, 0x46, 0x58);

pub const WEBSITE_URL: &str = "https://plexdio.kruxsys.com";
pub const PROJECT_URL: &str = "https://kruxsys.com/projects/plexdio/";
pub const SOURCE_URL: &str = "https://github.com/KruxSYS/plexdio";
pub const KRUXSYS_URL: &str = "https://kruxsys.com/";
pub const COPYRIGHT: &str = "© 2026 KruxSYS";

/// Maps the SVGs' 64-unit viewBox onto a square screen rect.
struct Grid {
    origin: Pos2,
    unit: f32,
}

impl Grid {
    fn new(rect: Rect) -> Self {
        let side = rect.width().min(rect.height());
        let origin = rect.center() - vec2(side, side) / 2.0;
        Self { origin, unit: side / 64.0 }
    }
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.unit
    }
    fn r(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
    fn w(&self, units: f32) -> f32 {
        units * self.unit
    }
}

/// The shared KruxSYS frame: `--panel` tile, `--line-2` border, cyan corner brackets.
fn frame(p: &Painter, g: &Grid) {
    p.rect(g.r(3.0, 3.0, 61.0, 61.0), 0.0, theme::BG, Stroke::new(g.w(2.0), theme::LINE_2), egui::StrokeKind::Middle);
    let s = Stroke::new(g.w(3.0), theme::ACCENT);
    // Square-capped L-shapes: extend each arm by half the stroke so the corners are filled.
    let h = 1.5;
    p.line_segment([g.p(3.0, 19.0), g.p(3.0, 3.0 - h)], s);
    p.line_segment([g.p(3.0 - h, 3.0), g.p(19.0, 3.0)], s);
    p.line_segment([g.p(45.0, 61.0), g.p(61.0 + h, 61.0)], s);
    p.line_segment([g.p(61.0, 61.0 + h), g.p(61.0, 45.0)], s);
}

/// Incenter and inradius of a triangle.
fn incircle(t: [Pos2; 3]) -> (Pos2, f32) {
    let a = (t[1] - t[2]).length();
    let b = (t[0] - t[2]).length();
    let c = (t[0] - t[1]).length();
    let per = a + b + c;
    let center = pos2((a * t[0].x + b * t[1].x + c * t[2].x) / per, (a * t[0].y + b * t[1].y + c * t[2].y) / per);
    let s = per / 2.0;
    let area = (s * (s - a) * (s - b) * (s - c)).max(0.0).sqrt();
    (center, area / s)
}

/// Offset every edge of a triangle by `d` (outwards when positive): a miter-joined outline edge.
fn offset_triangle(t: [Pos2; 3], d: f32) -> Vec<Pos2> {
    let (c, r) = incircle(t);
    let k = (r + d) / r;
    t.iter().map(|&q| c + (q - c) * k).collect()
}

/// The Plexdio mark in `rect` (drawn square, centred).
pub fn plexdio_mark(p: &Painter, rect: Rect) {
    let side = rect.width().min(rect.height());
    if side < SMALL {
        plexdio_mark_small(p, rect);
        return;
    }
    let g = Grid::new(rect);
    frame(p, &g);
    // Outlined play triangle: centre line (19,16) (19,48) (46,32), 7 units wide, miter joins.
    let tri = [g.p(19.0, 16.0), g.p(19.0, 48.0), g.p(46.0, 32.0)];
    let half = g.w(3.5);
    p.add(Shape::convex_polygon(offset_triangle(tri, half), theme::TEXT, Stroke::NONE));
    p.add(Shape::convex_polygon(offset_triangle(tri, -half), theme::BG, Stroke::NONE));
    // Signal square.
    p.rect_filled(g.r(25.0, 28.5, 32.0, 35.5), 0.0, theme::ACCENT);
}

/// Pixel-aligned small variant (title bar, credit line), matching `plexdio-16.svg`.
fn plexdio_mark_small(p: &Painter, rect: Rect) {
    let side = rect.width().min(rect.height()).round();
    let min = (rect.center() - vec2(side, side) / 2.0).round();
    let r = Rect::from_min_size(min, vec2(side, side));
    let u = side / 16.0;
    p.rect(r, 0.0, theme::BG, Stroke::new(1.0, SMALL_EDGE), egui::StrokeKind::Inside);
    let arm = (4.0 * u).round();
    let w = u.max(1.0).round();
    p.rect_filled(Rect::from_min_size(r.min, vec2(arm, w)), 0.0, theme::ACCENT);
    p.rect_filled(Rect::from_min_size(r.min, vec2(w, arm)), 0.0, theme::ACCENT);
    p.rect_filled(Rect::from_min_max(r.max - vec2(arm, w), r.max), 0.0, theme::ACCENT);
    p.rect_filled(Rect::from_min_max(r.max - vec2(w, arm), r.max), 0.0, theme::ACCENT);
    let at = |x: f32, y: f32| r.min + vec2(x, y) * u;
    p.add(Shape::convex_polygon(vec![at(5.0, 3.5), at(12.5, 8.0), at(5.0, 12.5)], theme::TEXT, Stroke::NONE));
}

/// One straight stroke of the KruxSYS chevron as a quad (butt ends, miter at the joint).
fn quad(p: &Painter, pts: [Pos2; 4], color: Color32) {
    p.add(Shape::convex_polygon(pts.to_vec(), color, Stroke::NONE));
}

/// The KruxSYS logo in `rect` (drawn square, centred).
pub fn kruxsys_mark(p: &Painter, rect: Rect) {
    let g = Grid::new(rect);
    frame(p, &g);
    // Cyan stem: M21 15v34, 7 wide.
    p.rect_filled(g.r(17.5, 15.0, 24.5, 49.0), 0.0, theme::ACCENT);
    // Light chevron: M45 15 L31 32 L45 49, 7 wide, miter join at (31, 32).
    let (a, j, b) = (g.p(45.0, 15.0), g.p(31.0, 32.0), g.p(45.0, 49.0));
    let half = g.w(3.5);
    let n1 = (j - a).normalized().rot90() * half;
    let n2 = (b - j).normalized().rot90() * half;
    // Miter points: where the offset lines of the two segments meet.
    let d1 = (j - a).normalized();
    let d2 = (b - j).normalized();
    let miter = |n1: egui::Vec2, n2: egui::Vec2| {
        // Solve (a + n1) + d1 * s = (j + n2) + d2 * t for the intersection.
        let p1 = a + n1;
        let p2 = j + n2;
        let cross = d1.x * d2.y - d1.y * d2.x;
        let s = ((p2.x - p1.x) * d2.y - (p2.y - p1.y) * d2.x) / cross;
        p1 + d1 * s
    };
    let (m_pos, m_neg) = (miter(n1, n2), miter(-n1, -n2));
    quad(p, [a + n1, m_pos, m_neg, a - n1], theme::TEXT);
    quad(p, [m_pos, b + n2, b - n2, m_neg], theme::TEXT);
    // Centre square.
    p.rect_filled(g.r(27.5, 28.5, 34.5, 35.5), 0.0, theme::ACCENT);
}

/// "Krux" + cyan monospace "SYS", as the site's wordmark. Returns the painted width.
pub fn paint_wordmark(p: &Painter, left_center: Pos2, size: f32) -> f32 {
    let krux = p.layout_no_wrap("Krux".into(), FontId::proportional(size), theme::TEXT);
    let sys = p.layout_no_wrap("SYS".into(), FontId::new(size * 0.92, FontFamily::Monospace), theme::ACCENT);
    let (kw, sw) = (krux.size().x, sys.size().x);
    let (kh, sh) = (krux.size().y, sys.size().y);
    p.galley(pos2(left_center.x, left_center.y - kh / 2.0), krux, theme::TEXT);
    p.galley(pos2(left_center.x + kw - 1.0, left_center.y - sh / 2.0), sys, theme::ACCENT);
    kw - 1.0 + sw
}

fn wordmark_width(ui: &egui::Ui, size: f32) -> f32 {
    ui.fonts_mut(|f| {
        f.layout_no_wrap("Krux".into(), FontId::proportional(size), theme::TEXT).size().x
            - 1.0
            + f.layout_no_wrap("SYS".into(), FontId::new(size * 0.92, FontFamily::Monospace), theme::ACCENT).size().x
    })
}

/// "by [KruxSYS mark] KruxSYS", centred in the available width. Clicking opens kruxsys.com.
pub fn credit(ui: &mut egui::Ui, size: f32) -> Response {
    let by = ui.fonts_mut(|f| f.layout_no_wrap("by".into(), FontId::proportional(size), theme::TEXT_DIM));
    let mark = (size * 1.25).round();
    let gap = (size * 0.4).round();
    let width = by.size().x + gap + mark + gap + wordmark_width(ui, size);
    let height = mark.max(by.size().y);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let p = ui.painter();
    let cy = rect.center().y;
    let mut x = rect.left();
    p.galley(pos2(x, cy - by.size().y / 2.0), by.clone(), theme::TEXT_DIM);
    x += by.size().x + gap;
    kruxsys_mark(p, Rect::from_min_size(pos2(x, cy - mark / 2.0), vec2(mark, mark)));
    x += mark + gap;
    paint_wordmark(p, pos2(x, cy), size);
    if resp.hovered() {
        p.line_segment([pos2(x, rect.bottom() + 1.0), pos2(rect.right(), rect.bottom() + 1.0)], Stroke::new(1.0, theme::ACCENT));
    }
    let resp = resp.on_hover_text("kruxsys.com").on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        crate::win::open_url(KRUXSYS_URL);
    }
    resp
}

/// Version line for Settings > About: "0.1.0 · build abc1234 · 2026-11-24".
pub fn build_line() -> String {
    format!("{} · build {} · {}", env!("CARGO_PKG_VERSION"), env!("PLEXDIO_BUILD_HASH"), env!("PLEXDIO_BUILD_DATE"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_triangle_moves_every_edge_by_d() {
        let t = [pos2(19.0, 16.0), pos2(19.0, 48.0), pos2(46.0, 32.0)];
        let out = offset_triangle(t, 3.5);
        // The vertical edge x = 19 moves to x = 15.5.
        assert!((out[0].x - 15.5).abs() < 1e-3 && (out[1].x - 15.5).abs() < 1e-3);
        let inner = offset_triangle(t, -3.5);
        assert!((inner[0].x - 22.5).abs() < 1e-3);
    }

    #[test]
    fn build_line_has_version() {
        assert!(build_line().starts_with(env!("CARGO_PKG_VERSION")));
    }
}
