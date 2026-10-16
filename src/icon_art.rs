// App icon, drawn procedurally so the repository needs no binary assets.
// Shared by the app (window + tray) and build.rs (the .exe icon), so it only uses `std`.

/// Renders a `size`×`size` RGBA icon: a rounded square in `accent` with a white fan.
pub fn render_icon(size: u32, accent: [u8; 3]) -> Vec<u8> {
    const SS: u32 = 4; // supersampling per axis
    let mut out = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    for py in 0..size {
        for px in 0..size {
            let (mut bg, mut fg) = (0u32, 0u32);
            for sy in 0..SS {
                for sx in 0..SS {
                    // Normalised coordinates in -1..1
                    let x = ((px * SS + sx) as f32 + 0.5) / (s * SS as f32) * 2.0 - 1.0;
                    let y = ((py * SS + sy) as f32 + 0.5) / (s * SS as f32) * 2.0 - 1.0;
                    if in_rounded_square(x, y, 0.94, 0.42) {
                        bg += 1;
                        if is_fan(x, y) {
                            fg += 1;
                        }
                    }
                }
            }
            let n = (SS * SS) as f32;
            let a_bg = bg as f32 / n;
            let a_fg = if bg > 0 { fg as f32 / bg as f32 } else { 0.0 };
            // Subtle vertical gradient on the background.
            let shade = 1.08 - 0.22 * (py as f32 / s);
            let i = ((py * size + px) * 4) as usize;
            for c in 0..3 {
                let base = (accent[c] as f32 * shade).min(255.0);
                out[i + c] = (base * (1.0 - a_fg) + 255.0 * a_fg).round() as u8;
            }
            out[i + 3] = (a_bg * 255.0).round() as u8;
        }
    }
    out
}

fn in_rounded_square(x: f32, y: f32, half: f32, radius: f32) -> bool {
    let dx = (x.abs() - (half - radius)).max(0.0);
    let dy = (y.abs() - (half - radius)).max(0.0);
    x.abs() <= half && y.abs() <= half && dx * dx + dy * dy <= radius * radius
}

fn is_fan(x: f32, y: f32) -> bool {
    let r = (x * x + y * y).sqrt();
    if r < 0.16 {
        return true; // hub
    }
    if !(0.16..=0.70).contains(&r) {
        return false;
    }
    let blades = 4.0;
    let sector = std::f32::consts::TAU / blades;
    // Swept blades: the angle offset grows with the radius.
    let theta = y.atan2(x) + r * 1.9;
    let local = theta.rem_euclid(sector) / sector; // 0..1 within one blade sector
    // Blade gets wider towards the tip.
    let width = 0.30 + 0.25 * ((r - 0.16) / 0.54);
    local < width
}
