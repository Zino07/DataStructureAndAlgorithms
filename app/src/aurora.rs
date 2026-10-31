//! Aurora Glass backdrop, painted once on the CPU (no runtime blur exists in
//! the renderer).
//!
//! * `backdrop`: the full-window aurora: a dark base, two or three soft
//!   colour fields from the accent, one curved light ribbon and fine grain.
//! * `frost`: a small, heavily blurred and slightly brightened copy. Each glass
//!   panel draws it aligned to the window, so a panel shows the frosted version
//!   of exactly what lies behind it.
//! * `grain`: not separate; it is baked into the backdrop at full resolution.
//!
//! Costs: 360 × 860 backdrop (1.2 MB RGBA) and a 90 × 215 frost (77 kB),
//! painted in a few milliseconds when the style or accent changes.

/// Logical size the images are painted for. Both are stretched to the
/// window, which is fine for soft content. The extra height is the travel of
/// the scroll parallax.
pub const WIDTH: u32 = 360;
pub const HEIGHT: u32 = 860;
const FROST_SCALE: u32 = 4;

#[derive(Clone, Copy)]
struct Rgb(f32, f32, f32);

impl Rgb {
    fn hex(v: u32) -> Rgb {
        Rgb(
            ((v >> 16) & 0xff) as f32 / 255.0,
            ((v >> 8) & 0xff) as f32 / 255.0,
            (v & 0xff) as f32 / 255.0,
        )
    }
}

/// Colour fields for one accent: (primary, secondary, deep).
fn scheme(palette: i32) -> (Rgb, Rgb, Rgb) {
    match palette {
        1 => (Rgb::hex(0xffd23b), Rgb::hex(0xff8a2a), Rgb::hex(0x3b2a06)),
        2 => (Rgb::hex(0xff6a2a), Rgb::hex(0xff2f6d), Rgb::hex(0x40140a)),
        3 => (Rgb::hex(0xa98bff), Rgb::hex(0x4f7bff), Rgb::hex(0x24124d)),
        _ => (Rgb::hex(0x9be62a), Rgb::hex(0x14c79a), Rgb::hex(0x06402a)),
    }
}

pub struct Images {
    pub width: u32,
    pub height: u32,
    pub backdrop: Vec<u8>,
    pub frost_width: u32,
    pub frost_height: u32,
    pub frost: Vec<u8>,
}

/// Soft field: 1 at the centre, smooth falloff (Gaussian) with radius `r`.
fn field(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> f32 {
    let dx = (x - cx) / rx;
    let dy = (y - cy) / ry;
    (-(dx * dx + dy * dy) * 2.2).exp()
}

/// Small deterministic hash noise in 0..1 (no RNG dependency).
fn noise(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

/// Linear-light colour of the aurora at a point (0..1 range, not clamped).
fn aurora_at(x: f32, y: f32, scheme: (Rgb, Rgb, Rgb)) -> Rgb {
    let (p, s, d) = scheme;
    let w = WIDTH as f32;
    let h = HEIGHT as f32;
    // base: near black with a faint cool lift at the top
    let mut c = Rgb(0.006, 0.008, 0.010);
    let mut add = |col: Rgb, k: f32| {
        c.0 += col.0 * k;
        c.1 += col.1 * k;
        c.2 += col.2 * k;
    };
    // primary bloom top-right, behind the hero card
    add(
        p,
        0.30 * field(x, y, w * 0.92, h * 0.07, w * 0.55, h * 0.20),
    );
    // deep pool on the left
    add(
        d,
        0.85 * field(x, y, w * 0.02, h * 0.45, w * 0.60, h * 0.28),
    );
    // secondary field low on the right
    add(s, 0.22 * field(x, y, w * 1.0, h * 0.72, w * 0.55, h * 0.20));
    // a dim primary echo near the bottom so the dock area is not flat
    add(p, 0.08 * field(x, y, w * 0.15, h * 1.0, w * 0.6, h * 0.14));
    // ribbon: a curved band of light across the upper third
    let curve = h * 0.30 + (x / w * std::f32::consts::PI * 1.3).sin() * h * 0.05 - x / w * h * 0.08;
    let band = ((y - curve) / (h * 0.035)).powi(2);
    let ribbon = (-band).exp() * (0.35 + 0.65 * field(x, y, w * 0.6, curve, w * 0.55, h * 0.4));
    add(p, 0.16 * ribbon);
    add(s, 0.08 * ribbon);
    // vignette
    let vx = (x / w - 0.5) * 2.0;
    let vy = (y / h - 0.45) * 2.0;
    let v = 1.0 - 0.28 * (vx * vx * 0.6 + vy * vy * 0.4).min(1.0);
    Rgb(c.0 * v, c.1 * v, c.2 * v)
}

fn to_srgb_u8(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5) as u8
}

/// Three box blurs approximate a Gaussian. Works in place on RGB floats.
fn blur(buf: &mut [Rgb], w: usize, h: usize, radius: usize) {
    let mut tmp = buf.to_vec();
    for _ in 0..3 {
        // horizontal
        for y in 0..h {
            for x in 0..w {
                let (mut r, mut g, mut b, mut n) = (0.0, 0.0, 0.0, 0.0);
                let lo = x.saturating_sub(radius);
                let hi = (x + radius).min(w - 1);
                for i in lo..=hi {
                    let p = buf[y * w + i];
                    r += p.0;
                    g += p.1;
                    b += p.2;
                    n += 1.0;
                }
                tmp[y * w + x] = Rgb(r / n, g / n, b / n);
            }
        }
        // vertical
        for x in 0..w {
            for y in 0..h {
                let (mut r, mut g, mut b, mut n) = (0.0, 0.0, 0.0, 0.0);
                let lo = y.saturating_sub(radius);
                let hi = (y + radius).min(h - 1);
                for i in lo..=hi {
                    let p = tmp[i * w + x];
                    r += p.0;
                    g += p.1;
                    b += p.2;
                    n += 1.0;
                }
                buf[y * w + x] = Rgb(r / n, g / n, b / n);
            }
        }
    }
}

pub fn render(palette: i32) -> Images {
    let sch = scheme(palette);
    let (w, h) = (WIDTH, HEIGHT);
    let mut backdrop = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let c = aurora_at(x as f32 + 0.5, y as f32 + 0.5, sch);
            // grain: about ±2% in sRGB, enough to stop banding in the dark
            let g = (noise(x, y) - 0.5) * 0.045;
            backdrop.push(to_srgb_u8(c.0 + g * 0.6));
            backdrop.push(to_srgb_u8(c.1 + g * 0.6));
            backdrop.push(to_srgb_u8(c.2 + g * 0.6));
            backdrop.push(255);
        }
    }

    // frost: sample the clean aurora at low resolution, blur, then lift and
    // saturate a little so blur does not grey the colours out
    let (fw, fh) = (w / FROST_SCALE, h / FROST_SCALE);
    let mut f: Vec<Rgb> = (0..fh)
        .flat_map(|y| {
            (0..fw).map(move |x| {
                let sx = (x * FROST_SCALE) as f32 + FROST_SCALE as f32 / 2.0;
                let sy = (y * FROST_SCALE) as f32 + FROST_SCALE as f32 / 2.0;
                aurora_at(sx, sy, sch)
            })
        })
        .collect();
    blur(&mut f, fw as usize, fh as usize, 4);
    let mut frost = Vec::with_capacity((fw * fh * 4) as usize);
    for c in &f {
        let l = 0.2126 * c.0 + 0.7152 * c.1 + 0.0722 * c.2;
        let sat = 1.25;
        let lift = 1.2;
        let px = |v: f32| ((l + (v - l) * sat) * lift + 0.006).max(0.0);
        frost.push(to_srgb_u8(px(c.0)));
        frost.push(to_srgb_u8(px(c.1)));
        frost.push(to_srgb_u8(px(c.2)));
        frost.push(255);
    }
    Images {
        width: w,
        height: h,
        backdrop,
        frost_width: fw,
        frost_height: fh,
        frost,
    }
}

#[cfg(test)]
/// sRGB channel (0..255) to linear.
fn lin(c: f32) -> f32 {
    let c = c / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
fn luminance(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

#[cfg(test)]
/// WCAG contrast ratio between two luminances.
pub fn contrast(a: f32, b: f32) -> f32 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
/// Composite `fg` (sRGB 0..255) with `alpha` over `bg`, the way the renderer
/// blends (in sRGB space).
fn over(fg: [f32; 3], alpha: f32, bg: [f32; 3]) -> [f32; 3] {
    [
        fg[0] * alpha + bg[0] * (1.0 - alpha),
        fg[1] * alpha + bg[1] * (1.0 - alpha),
        fg[2] * alpha + bg[2] * (1.0 - alpha),
    ]
}

#[cfg(test)]
/// Worst-case contrast of text on a glass panel anywhere on the backdrop:
/// panel = frost + dark tint (+ sheen at its brightest), text = white at
/// `text_alpha`. Returns the lowest ratio found.
pub fn worst_panel_contrast(
    img: &Images,
    tint: [f32; 3],
    tint_alpha: f32,
    glow: Option<([f32; 3], f32)>,
    text_alpha: f32,
) -> f32 {
    let sheen = 0x1a as f32 / 255.0;
    let mut worst = f32::MAX;
    for px in img.frost.chunks_exact(4) {
        let frost = [px[0] as f32, px[1] as f32, px[2] as f32];
        let mut panel = over(tint, tint_alpha, frost);
        if let Some((color, alpha)) = glow {
            panel = over(color, alpha, panel);
        }
        let panel = over([255.0; 3], sheen, panel);
        let text = over([247.0, 247.0, 244.0], text_alpha, panel);
        let ratio = contrast(
            luminance(text[0], text[1], text[2]),
            luminance(panel[0], panel[1], panel[2]),
        );
        worst = worst.min(ratio);
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;

    // Glass.tint in theme.slint: #0a0d10 at 0xa6 alpha; hero: 0x94.
    const TINT: [f32; 3] = [10.0, 13.0, 16.0];
    const ACCENTS: [[f32; 3]; 4] = [
        [212.0, 255.0, 58.0],
        [255.0, 229.0, 59.0],
        [255.0, 122.0, 47.0],
        [198.0, 179.0, 255.0],
    ];

    #[test]
    fn text_on_glass_meets_wcag_for_every_accent() {
        // tokens from theme.slint / bento.slint: panel tint 0xa6 (hero too),
        // accent panels 0xb3 with a 12% accent wash, hero glow 8%,
        // ink opaque, ink-muted 0xc4 white
        let a = |v: u32| v as f32 / 255.0;
        for palette in 0..4 {
            let img = render(palette);
            let accent = ACCENTS[palette as usize];
            let cases = [
                ("ink on panel", a(0xa6), None, 1.0, 7.0),
                ("muted on panel", a(0xa6), None, a(0xc4), 4.5),
                ("muted on hero", a(0xa6), Some((accent, 0.08)), a(0xc4), 4.5),
                (
                    "muted on accent panel",
                    a(0xb3),
                    Some((accent, 0.12)),
                    a(0xc4),
                    4.5,
                ),
            ];
            for (what, tint, glow, text, min) in cases {
                let ratio = worst_panel_contrast(&img, TINT, tint, glow, text);
                assert!(ratio >= min, "palette {palette}: {what} {ratio:.2} < {min}");
            }
        }
    }

    #[test]
    fn images_have_expected_sizes() {
        let img = render(0);
        assert_eq!(img.backdrop.len(), (img.width * img.height * 4) as usize);
        assert_eq!(
            img.frost.len(),
            (img.frost_width * img.frost_height * 4) as usize
        );
    }

    #[test]
    fn backdrop_is_dark_but_not_flat() {
        let img = render(0);
        let lums: Vec<f32> = img
            .backdrop
            .chunks_exact(4)
            .map(|p| luminance(p[0] as f32, p[1] as f32, p[2] as f32))
            .collect();
        let max = lums.iter().cloned().fold(0.0, f32::max);
        let min = lums.iter().cloned().fold(1.0, f32::min);
        assert!(max < 0.35, "too bright for dark glass: {max}");
        assert!(max - min > 0.04, "too flat to show any frosting");
    }
}
