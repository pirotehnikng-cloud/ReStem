use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let custom_icon = PathBuf::from("icon.ico");

        // Если в корне репозитория есть свой icon.ico — используем его, иначе генерируем логотип ReStem
        let icon_path = if custom_icon.exists() {
            custom_icon
        } else {
            let generated = out_dir.join("restem.ico");
            fs::write(&generated, generate_restem_ico()).expect("Failed to generate restem.ico");
            generated
        };

        let mut res = winresource::WindowsResource::new();
        res.set_icon(icon_path.to_str().unwrap());
        res.set("ProductName", "ReStem");
        res.set("FileDescription", "ReStem — Пакетное переименование файлов и папок");
        res.set("LegalCopyright", "Copyright © 2026 ReStem");
        res.compile().expect("Failed to compile Windows resources");
    }
}

/// Генерирует валидный 64x64 32-bit BGRA файл .ico с логотипом ReStem
fn generate_restem_ico() -> Vec<u8> {
    let w: usize = 64;
    let h: usize = 64;
    let xor_size = w * h * 4;
    let and_size = h * 8; // 64 бит (8 байт) на каждую строку маски прозрачности
    let dib_size = 40 + xor_size + and_size;

    let mut ico = Vec::with_capacity(22 + dib_size);

    // 1. Заголовок ICONDIR (6 байт)
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);

    // 2. Запись ICONDIRENTRY (16 байт)
    ico.push(w as u8);
    ico.push(h as u8);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&(dib_size as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());

    // 3. Заголовок BITMAPINFOHEADER (40 байт)
    ico.extend_from_slice(&40u32.to_le_bytes());
    ico.extend_from_slice(&(w as i32).to_le_bytes());
    ico.extend_from_slice(&((h * 2) as i32).to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes());
    ico.extend_from_slice(&((xor_size + and_size) as u32).to_le_bytes());
    ico.extend_from_slice(&[0u8; 16]);

    // 4. Пиксели изображения в порядке снизу вверх (BGRA)
    for y in (0..h).rev() {
        for x in 0..w {
            let sx = x / 2;
            let sy = y / 2;
            let dx = (sx as f32 - 15.5).abs();
            let dy = (sy as f32 - 15.5).abs();

            let mut r = 0u8;
            let mut g = 0u8;
            let mut b = 0u8;
            let mut a = 0u8;

            if dx < 14.5 && dy < 14.5 && (dx < 11.5 || dy < 11.5 || (dx - 11.5).hypot(dy - 11.5) <= 3.0) {
                r = 20;
                g = 30;
                b = 48;
                a = 255;

                let in_left_stem = (8..=12).contains(&sx) && (7..=24).contains(&sy);
                let in_top_bar = (12..=20).contains(&sx) && (7..=10).contains(&sy);
                let in_mid_bar = (12..=20).contains(&sx) && (14..=17).contains(&sy);
                let in_right_loop = (19..=23).contains(&sx) && (9..=15).contains(&sy);
                let in_leg = (17..=24).contains(&sy) && ((sx as i32 - (sy as i32 - 3)).abs() <= 2);

                if in_left_stem || in_top_bar || in_mid_bar || in_right_loop || in_leg {
                    let t = sy as f32 / 32.0;
                    r = (15.0 + t * 25.0) as u8;
                    g = (185.0 + t * 35.0) as u8;
                    b = (215.0 - t * 85.0) as u8;
                }
            }

            ico.extend_from_slice(&[b, g, r, a]);
        }
    }

    // 5. Маска AND (512 байт)
    ico.extend_from_slice(&vec![0u8; and_size]);
    ico
}
