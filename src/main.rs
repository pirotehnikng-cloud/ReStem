#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use regex::RegexBuilder;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_REPO_URL: &str = match option_env!("CARGO_PKG_REPOSITORY") {
    Some(url) if !url.is_empty() => url,
    _ => "https://github.com/",
};

fn main() -> eframe::Result<()> {
    let icon = build_app_icon();
    let cli_dir = env::args().nth(1).map(PathBuf::from).and_then(|p| {
        if p.is_dir() {
            Some(p)
        } else {
            p.parent().map(|par| par.to_path_buf())
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!(
                "ReStem v{} — Пакетное переименование файлов и папок",
                APP_VERSION
            ))
            .with_inner_size([1380.0, 840.0])
            .with_min_inner_size([1040.0, 620.0])
            .with_drag_and_drop(true)
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "ReStem",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            let mut app = ReStemApp::default();
            app.load_settings();
            apply_theme_and_style(&cc.egui_ctx, app.dark_mode);
            cc.egui_ctx.set_pixels_per_point(app.ui_scale);
            if let Some(dir) = cli_dir {
                app.open_new_directory(dir);
            }
            Ok(Box::new(app))
        }),
    )
}

/// Подключение системных шрифтов Windows (Segoe UI, Consolas, Segoe UI Emoji)
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    if let Ok(segoe_bytes) = fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts.font_data.insert(
            "segoe_ui".to_owned(),
            egui::FontData::from_owned(segoe_bytes).into(),
        );
        if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            vec.insert(0, "segoe_ui".to_owned());
        }
    }

    if let Ok(consolas_bytes) = fs::read(r"C:\Windows\Fonts\consola.ttf") {
        fonts.font_data.insert(
            "consolas".to_owned(),
            egui::FontData::from_owned(consolas_bytes).into(),
        );
        if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            vec.insert(0, "consolas".to_owned());
        }
    }

    for (key, path) in [
        ("segoe_emoji", r"C:\Windows\Fonts\seguiemj.ttf"),
        ("segoe_symbol", r"C:\Windows\Fonts\seguisym.ttf"),
    ] {
        if let Ok(bytes) = fs::read(path) {
            fonts
                .font_data
                .insert(key.to_owned(), egui::FontData::from_owned(bytes).into());
            if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                vec.push(key.to_owned());
            }
            if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                vec.push(key.to_owned());
            }
        }
    }

    ctx.set_fonts(fonts);
}

/// Настройка крупной типографики и высококонтрастной Светлой / Тёмной темы
fn apply_theme_and_style(ctx: &egui::Context, dark_mode: bool) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(17.5, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(14.5, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(12.5, egui::FontFamily::Proportional),
        ),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(7.5, 5.5);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    if dark_mode {
        visuals.override_text_color = Some(egui::Color32::from_rgb(235, 240, 245));
        visuals.panel_fill = egui::Color32::from_rgb(20, 23, 30);
        visuals.window_fill = egui::Color32::from_rgb(28, 32, 42);
        visuals.extreme_bg_color = egui::Color32::from_rgb(14, 16, 22);
        visuals.faint_bg_color = egui::Color32::from_rgb(26, 31, 40);
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(48, 55, 70));
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(38, 44, 56);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 60, 78);
        visuals.selection.bg_fill = egui::Color32::from_rgb(25, 110, 195);
    } else {
        visuals.override_text_color = Some(egui::Color32::from_rgb(22, 26, 34));
        visuals.panel_fill = egui::Color32::from_rgb(242, 244, 248);
        visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
        visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
        visuals.faint_bg_color = egui::Color32::from_rgb(233, 237, 244);
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(210, 216, 226));
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(228, 233, 241);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(210, 220, 235);
        visuals.selection.bg_fill = egui::Color32::from_rgb(18, 118, 215);
    }

    style.visuals = visuals;
    ctx.set_style(style);
}

fn card_frame(dark_mode: bool) -> egui::Frame {
    let (fill, stroke_col) = if dark_mode {
        (
            egui::Color32::from_rgb(30, 35, 45),
            egui::Color32::from_rgb(48, 56, 72),
        )
    } else {
        (
            egui::Color32::from_rgb(255, 255, 255),
            egui::Color32::from_rgb(218, 224, 234),
        )
    };

    egui::Frame::none()
        .fill(fill)
        .rounding(8.0)
        .stroke(egui::Stroke::new(1.0_f32, stroke_col))
        .inner_margin(egui::Margin::same(10.0))
}

fn section_title(ui: &mut egui::Ui, text: &str, dark_mode: bool) {
    let color = if dark_mode {
        egui::Color32::from_rgb(95, 175, 255)
    } else {
        egui::Color32::from_rgb(15, 90, 185)
    };
    ui.label(egui::RichText::new(text).strong().size(15.0).color(color));
}

fn build_app_icon() -> egui::IconData {
    let width = 32;
    let height = 32;
    let mut rgba = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            let dx = (x as f32 - 15.5).abs();
            let dy = (y as f32 - 15.5).abs();

            if dx < 14.5
                && dy < 14.5
                && (dx < 11.5 || dy < 11.5 || (dx - 11.5).hypot(dy - 11.5) <= 3.0)
            {
                rgba[idx] = 20;
                rgba[idx + 1] = 30;
                rgba[idx + 2] = 48;
                rgba[idx + 3] = 255;

                let in_left_stem = (8..=12).contains(&x) && (7..=24).contains(&y);
                let in_top_bar = (12..=20).contains(&x) && (7..=10).contains(&y);
                let in_mid_bar = (12..=20).contains(&x) && (14..=17).contains(&y);
                let in_right_loop = (19..=23).contains(&x) && (9..=15).contains(&y);
                let in_leg = (17..=24).contains(&y) && ((x as i32 - (y as i32 - 3)).abs() <= 2);

                if in_left_stem || in_top_bar || in_mid_bar || in_right_loop || in_leg {
                    let t = y as f32 / 32.0;
                    rgba[idx] = (15.0 + t * 25.0) as u8;
                    rgba[idx + 1] = (185.0 + t * 35.0) as u8;
                    rgba[idx + 2] = (215.0 - t * 85.0) as u8;
                    rgba[idx + 3] = 255;
                }
            }
        }
    }

    egui::IconData {
        rgba,
        width: width as u32,
        height: height as u32,
    }
}

/// Умная («натуральная») сортировка строк с учётом чисел (file2 перед file10)
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ia = a.chars().peekable();
    let mut ib = b.chars().peekable();

    while let (Some(&ca), Some(&cb)) = (ia.peek(), ib.peek()) {
        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            let mut na = String::new();
            while let Some(&c) = ia.peek() {
                if c.is_ascii_digit() {
                    na.push(c);
                    ia.next();
                } else {
                    break;
                }
            }
            let mut nb = String::new();
            while let Some(&c) = ib.peek() {
                if c.is_ascii_digit() {
                    nb.push(c);
                    ib.next();
                } else {
                    break;
                }
            }
            let va = na.trim_start_matches('0');
            let vb = nb.trim_start_matches('0');
            let ord = va
                .len()
                .cmp(&vb.len())
                .then_with(|| va.cmp(vb))
                .then_with(|| na.len().cmp(&nb.len()));
            if ord != Ordering::Equal {
                return ord;
            }
        } else {
            let ua = ca.to_lowercase().next().unwrap_or(ca);
            let ub = cb.to_lowercase().next().unwrap_or(cb);
            let ord = ua.cmp(&ub);
            if ord != Ordering::Equal {
                return ord;
            }
            ia.next();
            ib.next();
        }
    }
    ia.count().cmp(&ib.count())
}

/// Разбор UNIX-времени на (год, месяц, день, часы, минуты)
fn decompose_timestamp(secs: u64) -> (i64, u32, u32, u64, u64) {
    if secs == 0 {
        return (1970, 1, 1, 0, 0);
    }
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let hours = rem / 3600;
    let mins = (rem % 3600) / 60;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    (year, m, d, hours, mins)
}

fn format_timestamp(secs: u64) -> String {
    if secs == 0 {
        return "—".to_string();
    }
    let (year, m, d, hours, mins) = decompose_timestamp(secs);
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, m, d, hours, mins)
}

/// Прямая транслитерация кириллицы (RU -> LAT)
fn transliterate_ru_to_lat(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for ch in input.chars() {
        let replacement = match ch {
            'а' => "a",    'А' => "A",
            'б' => "b",    'Б' => "B",
            'в' => "v",    'В' => "V",
            'г' => "g",    'Г' => "G",
            'д' => "d",    'Д' => "D",
            'е' => "e",    'Е' => "E",
            'ё' => "yo",   'Ё' => "Yo",
            'ж' => "zh",   'Ж' => "Zh",
            'з' => "z",    'З' => "Z",
            'и' => "i",    'И' => "I",
            'й' => "y",    'Й' => "Y",
            'к' => "k",    'К' => "K",
            'л' => "l",    'Л' => "L",
            'м' => "m",    'М' => "M",
            'н' => "n",    'Н' => "N",
            'о' => "o",    'О' => "O",
            'п' => "p",    'П' => "P",
            'р' => "r",    'Р' => "R",
            'с' => "s",    'С' => "S",
            'т' => "t",    'Т' => "T",
            'у' => "u",    'У' => "U",
            'ф' => "f",    'Ф' => "F",
            'х' => "kh",   'Х' => "Kh",
            'ц' => "ts",   'Ц' => "Ts",
            'ч' => "ch",   'Ч' => "Ch",
            'ш' => "sh",   'Ш' => "Sh",
            'щ' => "shch", 'Щ' => "Shch",
            'ъ' | 'Ъ' | 'ь' | 'Ь' => "",
            'ы' => "y",    'Ы' => "Y",
            'э' => "e",    'Э' => "E",
            'ю' => "yu",   'Ю' => "Yu",
            'я' => "ya",   'Я' => "Ya",
            _ => {
                out.push(ch);
                continue;
            }
        };
        out.push_str(replacement);
    }
    out
}

/// Обратная транслитерация латиницы в кириллицу (LAT -> RU)
fn transliterate_lat_to_ru(input: &str) -> String {
    let mut s = input.to_string();
    let multi_rules = [
        ("shch", "щ"), ("Shch", "Щ"), ("SHCH", "Щ"),
        ("yo", "ё"),   ("Yo", "Ё"),   ("YO", "Ё"),
        ("zh", "ж"),   ("Zh", "Ж"),   ("ZH", "Ж"),
        ("kh", "х"),   ("Kh", "Х"),   ("KH", "Х"),
        ("ts", "ц"),   ("Ts", "Ц"),   ("TS", "Ц"),
        ("ch", "ч"),   ("Ch", "Ч"),   ("CH", "Ч"),
        ("sh", "ш"),   ("Sh", "Ш"),   ("SH", "Ш"),
        ("yu", "ю"),   ("Yu", "Ю"),   ("YU", "Ю"),
        ("ya", "я"),   ("Ya", "Я"),   ("YA", "Я"),
    ];
    for (pat, rep) in multi_rules {
        s = s.replace(pat, rep);
    }

    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        let rep = match ch {
            'a' => 'а', 'A' => 'А',
            'b' => 'б', 'B' => 'Б',
            'v' | 'w' => 'в', 'V' | 'W' => 'В',
            'g' => 'г', 'G' => 'Г',
            'd' => 'д', 'D' => 'Д',
            'e' => 'е', 'E' => 'Е',
            'z' => 'з', 'Z' => 'З',
            'i' => 'и', 'I' => 'И',
            'y' | 'j' => 'й', 'Y' | 'J' => 'Й',
            'k' | 'q' => 'к', 'K' | 'Q' => 'К',
            'l' => 'л', 'L' => 'Л',
            'm' => 'м', 'M' => 'М',
            'n' => 'н', 'N' => 'Н',
            'o' => 'о', 'O' => 'О',
            'p' => 'п', 'P' => 'П',
            'r' => 'р', 'R' => 'Р',
            's' => 'с', 'S' => 'С',
            't' => 'т', 'T' => 'Т',
            'u' => 'у', 'U' => 'У',
            'f' => 'ф', 'F' => 'Ф',
            'h' => 'х', 'H' => 'Х',
            'c' => 'ц', 'C' => 'Ц',
            _ => ch,
        };
        out.push(rep);
    }
    out
}

/// Санация («чистка») основы имени файла
fn sanitize_stem(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut prev_space = false;

    for ch in input.chars() {
        if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            continue;
        }
        if ch.is_whitespace() {
            if !prev_space {
                cleaned.push(' ');
                prev_space = true;
            }
        } else {
            cleaned.push(ch);
            prev_space = false;
        }
    }
    cleaned.trim().trim_end_matches('.').trim().to_string()
}

/// Встроенный быстрый парсер EXIF из заголовка JPEG (извлекает Дату съёмки и Модель камеры)
fn extract_jpeg_exif(path: &Path) -> (Option<String>, Option<String>) {
    let Ok(mut file) = fs::File::open(path) else {
        return (None, None);
    };
    let mut buf = vec![0u8; 65536];
    let Ok(n) = file.read(&mut buf) else {
        return (None, None);
    };
    buf.truncate(n);

    if buf.len() < 4 || buf[0] != 0xFF || buf[1] != 0xD8 {
        return (None, None);
    }

    let mut pos = 2;
    while pos + 4 <= buf.len() {
        if buf[pos] != 0xFF {
            break;
        }
        let marker = buf[pos + 1];
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let seg_len = u16::from_be_bytes([buf[pos + 2], buf[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > buf.len() {
            break;
        }
        if marker == 0xE1 && seg_len >= 14 {
            let seg = &buf[pos + 4..pos + 2 + seg_len];
            if seg.starts_with(b"Exif\0\0") {
                return parse_tiff_exif(&seg[6..]);
            }
        }
        pos += 2 + seg_len;
    }
    (None, None)
}

fn parse_tiff_exif(tiff: &[u8]) -> (Option<String>, Option<String>) {
    if tiff.len() < 8 {
        return (None, None);
    }
    let le = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return (None, None),
    };
    let r16 = |offset: usize| -> Option<u16> {
        let b = tiff.get(offset..offset + 2)?;
        Some(if le {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    };
    let r32 = |offset: usize| -> Option<u32> {
        let b = tiff.get(offset..offset + 4)?;
        Some(if le {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    let read_ascii = |val_offset: usize, count: usize| -> Option<String> {
        let slice = tiff.get(val_offset..val_offset + count)?;
        let s = String::from_utf8_lossy(slice)
            .trim_matches(char::from(0))
            .trim()
            .to_string();
        if s.is_empty() { None } else { Some(s) }
    };

    let Some(ifd0_off) = r32(4).map(|v| v as usize) else {
        return (None, None);
    };

    let mut date_str: Option<String> = None;
    let mut camera_model: Option<String> = None;
    let mut exif_sub_ifd: Option<usize> = None;

    if let Some(entries) = r16(ifd0_off) {
        for i in 0..entries as usize {
            let entry_off = ifd0_off + 2 + i * 12;
            let (Some(tag), Some(dtype), Some(count), Some(val_raw)) = (
                r16(entry_off),
                r16(entry_off + 2),
                r32(entry_off + 4),
                r32(entry_off + 8),
            ) else {
                break;
            };
            if tag == 0x8769 {
                exif_sub_ifd = Some(val_raw as usize);
            } else if dtype == 2 && (tag == 0x0110 || tag == 0x0132) {
                let c = count as usize;
                let str_off = if c <= 4 { entry_off + 8 } else { val_raw as usize };
                if let Some(val) = read_ascii(str_off, c) {
                    if tag == 0x0110 {
                        camera_model = Some(
                            val.chars()
                                .map(|ch| if ch.is_alphanumeric() || ch == '-' { ch } else { '_' })
                                .collect::<String>()
                                .trim_matches('_')
                                .to_string(),
                        );
                    } else if tag == 0x0132 && date_str.is_none() {
                        date_str = format_exif_date(&val);
                    }
                }
            }
        }
    }

    if let Some(sub_off) = exif_sub_ifd {
        if let Some(entries) = r16(sub_off) {
            for i in 0..entries as usize {
                let entry_off = sub_off + 2 + i * 12;
                let (Some(tag), Some(dtype), Some(count), Some(val_raw)) = (
                    r16(entry_off),
                    r16(entry_off + 2),
                    r32(entry_off + 4),
                    r32(entry_off + 8),
                ) else {
                    break;
                };
                if dtype == 2 && (tag == 0x9003 || tag == 0x9004) {
                    let c = count as usize;
                    let str_off = if c <= 4 { entry_off + 8 } else { val_raw as usize };
                    if let Some(val) = read_ascii(str_off, c) {
                        if let Some(formatted) = format_exif_date(&val) {
                            date_str = Some(formatted);
                            break;
                        }
                    }
                }
            }
        }
    }

    (date_str, camera_model)
}

fn format_exif_date(raw: &str) -> Option<String> {
    if raw.len() >= 10 {
        let d = &raw[0..10];
        let clean: String = d
            .chars()
            .map(|c| if c == ':' { '-' } else { c })
            .collect();
        if clean.chars().take(4).all(|c| c.is_ascii_digit()) {
            return Some(clean);
        }
    }
    None
}

/// Построение LayoutJob с посимвольной подсветкой изменений (Diff)
fn build_diff_layout_job(
    old_name: &str,
    new_name: &str,
    is_new_column: bool,
    dark_mode: bool,
) -> egui::text::LayoutJob {
    let old_chars: Vec<char> = old_name.chars().collect();
    let new_chars: Vec<char> = new_name.chars().collect();

    let mut pref = 0;
    while pref < old_chars.len()
        && pref < new_chars.len()
        && old_chars[pref] == new_chars[pref]
    {
        pref += 1;
    }

    let mut suff = 0;
    while suff < (old_chars.len() - pref)
        && suff < (new_chars.len() - pref)
        && old_chars[old_chars.len() - 1 - suff] == new_chars[new_chars.len() - 1 - suff]
    {
        suff += 1;
    }

    let base_fg = if dark_mode {
        egui::Color32::from_rgb(230, 235, 242)
    } else {
        egui::Color32::from_rgb(25, 30, 40)
    };

    let font_id = egui::FontId::new(14.5, egui::FontFamily::Proportional);
    let base_fmt = egui::TextFormat {
        font_id: font_id.clone(),
        color: base_fg,
        ..Default::default()
    };

    let mut job = egui::text::LayoutJob::default();

    if is_new_column {
        let (hl_bg, hl_fg) = if dark_mode {
            (
                egui::Color32::from_rgb(20, 95, 62),
                egui::Color32::from_rgb(145, 255, 195),
            )
        } else {
            (
                egui::Color32::from_rgb(195, 242, 215),
                egui::Color32::from_rgb(8, 95, 45),
            )
        };

        let p1: String = new_chars[..pref].iter().collect();
        let p2: String = new_chars[pref..new_chars.len() - suff].iter().collect();
        let p3: String = new_chars[new_chars.len() - suff..].iter().collect();

        if !p1.is_empty() {
            job.append(&p1, 0.0, base_fmt.clone());
        }
        if !p2.is_empty() {
            job.append(
                &p2,
                0.0,
                egui::TextFormat {
                    font_id: font_id.clone(),
                    color: hl_fg,
                    background: hl_bg,
                    ..Default::default()
                },
            );
        }
        if !p3.is_empty() {
            job.append(&p3, 0.0, base_fmt);
        }
    } else {
        let (del_bg, del_fg) = if dark_mode {
            (
                egui::Color32::from_rgb(90, 30, 35),
                egui::Color32::from_rgb(255, 155, 155),
            )
        } else {
            (
                egui::Color32::from_rgb(252, 215, 215),
                egui::Color32::from_rgb(155, 20, 25),
            )
        };

        let p1: String = old_chars[..pref].iter().collect();
        let p2: String = old_chars[pref..old_chars.len() - suff].iter().collect();
        let p3: String = old_chars[old_chars.len() - suff..].iter().collect();

        if !p1.is_empty() {
            job.append(&p1, 0.0, base_fmt.clone());
        }
        if !p2.is_empty() {
            job.append(
                &p2,
                0.0,
                egui::TextFormat {
                    font_id: font_id.clone(),
                    color: del_fg,
                    background: del_bg,
                    strikethrough: egui::Stroke::new(1.0_f32, del_fg),
                    ..Default::default()
                },
            );
        }
        if !p3.is_empty() {
            job.append(&p3, 0.0, base_fmt);
        }
    }

    job
}

fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "ico" | "gif"
    )
}

fn open_in_system(path: &Path) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer").arg(path).spawn();
    }
}

fn open_url_in_browser(url: &str) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer").arg(url).spawn();
    }
}

fn settings_file_path() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join("ReStem");
        let _ = fs::create_dir_all(&dir);
        dir.join("restem_settings.ini")
    } else {
        PathBuf::from("restem_settings.ini")
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Lang {
    Ru,
    En,
}

impl Lang {
    fn tr(self, ru: &'static str, en: &'static str) -> &'static str {
        match self {
            Lang::Ru => ru,
            Lang::En => en,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum SortField {
    NaturalName,
    AlphaName,
    ModifiedDate,
    Size,
    Extension,
    FolderPath,
}

#[derive(PartialEq, Clone, Copy)]
enum StatusFilter {
    All,
    OnlyChanged,
    OnlyIncluded,
    OnlyUnchanged,
}

#[derive(PartialEq, Clone, Copy)]
enum SizeFilterMode {
    Any,
    GreaterThanKB,
    LessThanKB,
    BetweenKB,
}

#[derive(PartialEq, Clone, Copy)]
enum CaseMode {
    None,
    Lower,
    Upper,
    Capitalize,
    Title,
}

#[derive(PartialEq, Clone, Copy)]
enum TranslitMode {
    None,
    RuToLat,
    LatToRu,
}

#[derive(PartialEq, Clone, Copy)]
enum NumMode {
    None,
    Suffix,
    Prefix,
}

#[derive(PartialEq, Clone, Copy)]
enum ExtMode {
    Keep,
    Lower,
    Upper,
    Remove,
}

#[derive(Clone)]
struct UserPreset {
    title: String,
    find_str: String,
    replace_str: String,
    use_regex: bool,
    prefix: String,
    suffix: String,
    translit_mode: TranslitMode,
    case_mode: CaseMode,
    sanitize: bool,
}

enum PreviewState {
    Empty,
    Folder {
        name: String,
        items_count: usize,
    },
    Image {
        name: String,
        texture: egui::TextureHandle,
        orig_w: u32,
        orig_h: u32,
        exif_info: String,
        tex_size: egui::Vec2,
    },
    Text {
        name: String,
        content: String,
        truncated: bool,
    },
    BinaryOrOther {
        name: String,
        info: String,
    },
}

#[derive(Clone)]
struct FsEntry {
    path: PathBuf,
    parent_dir: PathBuf,
    parent_name: String,
    rel_parent: String,
    name: String,
    ext_lower: String,
    is_dir: bool,
    is_hidden: bool,
    depth: usize,
    size: u64,
    modified_ts: u64,
    img_dims: Option<(u32, u32)>,
    exif_date: Option<String>,
    exif_camera: Option<String>,
}

#[derive(Clone)]
struct PreviewRow {
    old_path: PathBuf,
    parent_dir: PathBuf,
    rel_parent: String,
    is_dir: bool,
    depth: usize,
    size: u64,
    modified_ts: u64,
    old_name: String,
    new_name: String,
    included: bool,
    changed: bool,
}

struct ReStemApp {
    lang: Lang,
    dark_mode: bool,
    ui_scale: f32,
    inline_diff: bool,

    current_dir: Option<PathBuf>,
    subfolders: Vec<PathBuf>,
    all_entries: Vec<FsEntry>,
    preview_rows: Vec<PreviewRow>,
    excluded_paths: HashSet<PathBuf>,

    undo_stack: Vec<Vec<(PathBuf, PathBuf)>>,

    selected_path: Option<PathBuf>,
    preview_state: PreviewState,
    confirm_trash_modal: bool,
    show_about_modal: bool,
    show_regex_help_modal: bool,
    show_list_rename_modal: bool,

    user_presets: Vec<UserPreset>,
    new_preset_title: String,

    sort_field: SortField,
    sort_asc: bool,
    folders_first: bool,

    search_query: String,
    search_regex: bool,
    search_case_sensitive: bool,
    status_filter: StatusFilter,
    include_files: bool,
    include_dirs: bool,
    recursive: bool,
    hide_system_hidden: bool,

    ext_filter: String,
    ext_exclude_filter: String,
    exclude_words_filter: String,
    size_filter_mode: SizeFilterMode,
    size_min_kb: u64,
    size_max_kb: u64,

    find_str: String,
    replace_str: String,
    use_regex: bool,
    case_sensitive: bool,

    case_mode: CaseMode,
    translit_mode: TranslitMode,
    sanitize_names: bool,

    prefix: String,
    suffix: String,
    trim_start: usize,
    trim_end: usize,
    insert_text: String,
    insert_pos: usize,
    insert_from_end: bool,

    use_custom_list: bool,
    custom_list_raw: String,

    num_mode: NumMode,
    num_start: usize,
    num_step: usize,
    num_pad: usize,
    num_sep: String,
    reset_num_per_dir: bool,

    ext_mode: ExtMode,
    new_ext: String,

    status_msg: String,
}

impl Default for ReStemApp {
    fn default() -> Self {
        Self {
            lang: Lang::Ru,
            dark_mode: false,
            ui_scale: 1.0,
            inline_diff: true,

            current_dir: None,
            subfolders: Vec::new(),
            all_entries: Vec::new(),
            preview_rows: Vec::new(),
            excluded_paths: HashSet::new(),
            undo_stack: Vec::new(),

            selected_path: None,
            preview_state: PreviewState::Empty,
            confirm_trash_modal: false,
            show_about_modal: false,
            show_regex_help_modal: false,
            show_list_rename_modal: false,

            user_presets: Vec::new(),
            new_preset_title: String::new(),

            sort_field: SortField::NaturalName,
            sort_asc: true,
            folders_first: true,

            search_query: String::new(),
            search_regex: false,
            search_case_sensitive: false,
            status_filter: StatusFilter::All,
            include_files: true,
            include_dirs: true,
            recursive: false,
            hide_system_hidden: true,

            ext_filter: String::new(),
            ext_exclude_filter: String::new(),
            exclude_words_filter: String::new(),
            size_filter_mode: SizeFilterMode::Any,
            size_min_kb: 100,
            size_max_kb: 10240,

            find_str: String::new(),
            replace_str: String::new(),
            use_regex: false,
            case_sensitive: true,

            case_mode: CaseMode::None,
            translit_mode: TranslitMode::None,
            sanitize_names: false,

            prefix: String::new(),
            suffix: String::new(),
            trim_start: 0,
            trim_end: 0,
            insert_text: String::new(),
            insert_pos: 0,
            insert_from_end: false,

            use_custom_list: false,
            custom_list_raw: String::new(),

            num_mode: NumMode::None,
            num_start: 1,
            num_step: 1,
            num_pad: 2,
            num_sep: "_".to_string(),
            reset_num_per_dir: true,

            ext_mode: ExtMode::Keep,
            new_ext: String::new(),
            status_msg: "Выберите папку или перетащите её мышью в окно программы.".to_string(),
        }
    }
}

impl ReStemApp {
    fn save_settings(&self) {
        let mut lines = Vec::new();
        lines.push(format!("lang={}", if self.lang == Lang::Ru { "ru" } else { "en" }));
        lines.push(format!("dark_mode={}", self.dark_mode));
        lines.push(format!("ui_scale={:.2}", self.ui_scale));
        lines.push(format!("inline_diff={}", self.inline_diff));

        for p in &self.user_presets {
            let t_code = match p.translit_mode {
                TranslitMode::None => 0,
                TranslitMode::RuToLat => 1,
                TranslitMode::LatToRu => 2,
            };
            let c_code = match p.case_mode {
                CaseMode::None => 0,
                CaseMode::Lower => 1,
                CaseMode::Upper => 2,
                CaseMode::Capitalize => 3,
                CaseMode::Title => 4,
            };
            lines.push(format!(
                "preset={}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                p.title.replace('\t', " "),
                p.find_str.replace('\t', " "),
                p.replace_str.replace('\t', " "),
                p.use_regex,
                p.prefix.replace('\t', " "),
                p.suffix.replace('\t', " "),
                t_code,
                c_code,
                p.sanitize
            ));
        }

        let _ = fs::write(settings_file_path(), lines.join("\n"));
    }

    fn load_settings(&mut self) {
        let Ok(content) = fs::read_to_string(settings_file_path()) else {
            return;
        };
        self.user_presets.clear();
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("lang=") {
                self.lang = if val.trim() == "en" { Lang::En } else { Lang::Ru };
            } else if let Some(val) = line.strip_prefix("dark_mode=") {
                self.dark_mode = val.trim() == "true";
            } else if let Some(val) = line.strip_prefix("ui_scale=") {
                if let Ok(sc) = val.trim().parse::<f32>() {
                    self.ui_scale = sc.clamp(0.85, 1.5);
                }
            } else if let Some(val) = line.strip_prefix("inline_diff=") {
                self.inline_diff = val.trim() == "true";
            } else if let Some(val) = line.strip_prefix("preset=") {
                let parts: Vec<&str> = val.split('\t').collect();
                if parts.len() == 9 {
                    let translit_mode = match parts[6] {
                        "1" => TranslitMode::RuToLat,
                        "2" => TranslitMode::LatToRu,
                        _ => TranslitMode::None,
                    };
                    let case_mode = match parts[7] {
                        "1" => CaseMode::Lower,
                        "2" => CaseMode::Upper,
                        "3" => CaseMode::Capitalize,
                        "4" => CaseMode::Title,
                        _ => CaseMode::None,
                    };
                    self.user_presets.push(UserPreset {
                        title: parts[0].to_string(),
                        find_str: parts[1].to_string(),
                        replace_str: parts[2].to_string(),
                        use_regex: parts[3] == "true",
                        prefix: parts[4].to_string(),
                        suffix: parts[5].to_string(),
                        translit_mode,
                        case_mode,
                        sanitize: parts[8] == "true",
                    });
                }
            }
        }
    }

    fn register_explorer_context_menu(&mut self, install: bool) {
        #[cfg(windows)]
        {
            if install {
                if let Ok(exe_path) = env::current_exe() {
                    let exe_str = exe_path.display().to_string();
                    let cmd_str = format!("\"{}\" \"%1\"", exe_str);
                    let title = self
                        .lang
                        .tr("Открыть в ReStem", "Open in ReStem");

                    let _ = Command::new("reg")
                        .args([
                            "add",
                            r"HKCU\Software\Classes\Directory\shell\ReStem",
                            "/ve",
                            "/d",
                            title,
                            "/f",
                        ])
                        .output();
                    let _ = Command::new("reg")
                        .args([
                            "add",
                            r"HKCU\Software\Classes\Directory\shell\ReStem",
                            "/v",
                            "Icon",
                            "/d",
                            &exe_str,
                            "/f",
                        ])
                        .output();
                    let status = Command::new("reg")
                        .args([
                            "add",
                            r"HKCU\Software\Classes\Directory\shell\ReStem\command",
                            "/ve",
                            "/d",
                            &cmd_str,
                            "/f",
                        ])
                        .status();

                    if status.map(|s| s.success()).unwrap_or(false) {
                        self.status_msg = self
                            .lang
                            .tr(
                                "Пункт «Открыть в ReStem» успешно добавлен в меню Проводника Windows!",
                                "'Open in ReStem' successfully added to Windows Explorer context menu!",
                            )
                            .to_string();
                    }
                }
            } else {
                let _ = Command::new("reg")
                    .args([
                        "delete",
                        r"HKCU\Software\Classes\Directory\shell\ReStem",
                        "/f",
                    ])
                    .output();
                self.status_msg = self
                    .lang
                    .tr(
                        "Пункт «Открыть в ReStem» удалён из контекстного меню Проводника.",
                        "'Open in ReStem' removed from Windows Explorer context menu.",
                    )
                    .to_string();
            }
        }
    }

    fn export_rollback_script(&mut self) {
        let Some(last_batch) = self.undo_stack.last() else {
            return;
        };
        if let Some(save_path) = rfd::FileDialog::new()
            .set_file_name("restem_rollback.bat")
            .add_filter("Batch Script (.bat)", &["bat"])
            .save_file()
        {
            let mut lines = vec![
                "@echo off".to_string(),
                "chcp 65001 >nul".to_string(),
                "echo Restoring filenames via ReStem rollback script...".to_string(),
            ];
            for (curr_path, orig_path) in last_batch.iter().rev() {
                if let Some(orig_name) = orig_path.file_name() {
                    lines.push(format!(
                        "ren \"{}\" \"{}\"",
                        curr_path.display(),
                        orig_name.to_string_lossy()
                    ));
                }
            }
            lines.push("echo Done!".to_string());
            lines.push("pause".to_string());

            if fs::write(&save_path, lines.join("\r\n")).is_ok() {
                self.status_msg = match self.lang {
                    Lang::Ru => format!("Скрипт отката сохранён: {}", save_path.display()),
                    Lang::En => format!("Rollback script saved: {}", save_path.display()),
                };
            }
        }
    }

    fn format_size(bytes: u64, is_dir: bool) -> String {
        if is_dir {
            return "—".to_string();
        }
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }

    fn is_entry_hidden(path: &Path, meta: Option<&fs::Metadata>) -> bool {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with('.')
            || name.eq_ignore_ascii_case("node_modules")
            || name.eq_ignore_ascii_case("$RECYCLE.BIN")
            || name.eq_ignore_ascii_case("System Volume Information")
        {
            return true;
        }
        #[cfg(windows)]
        if let Some(m) = meta {
            use std::os::windows::fs::MetadataExt;
            let attrs = m.file_attributes();
            if (attrs & 0x2) != 0 || (attrs & 0x4) != 0 {
                return true;
            }
        }
        #[cfg(not(windows))]
        let _ = meta;

        false
    }

    fn select_for_preview(&mut self, path: PathBuf, ctx: &egui::Context) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.selected_path = Some(path.clone());

        if path.is_dir() {
            let count = fs::read_dir(&path)
                .map(|rd| rd.flatten().count())
                .unwrap_or(0);
            self.preview_state = PreviewState::Folder {
                name,
                items_count: count,
            };
            return;
        }

        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        if is_image_ext(&ext) {
            let (exif_d, exif_cam) = extract_jpeg_exif(&path);
            let mut exif_parts = Vec::new();
            if let Some(d) = exif_d {
                exif_parts.push(format!("📅 {}", d));
            }
            if let Some(c) = exif_cam {
                exif_parts.push(format!("📷 {}", c));
            }
            let exif_info = exif_parts.join("  •  ");

            match image::open(&path) {
                Ok(img) => {
                    let orig_w = img.width();
                    let orig_h = img.height();
                    let thumb = img.thumbnail(320, 320);
                    let rgba = thumb.to_rgba8();
                    let tw = rgba.width() as usize;
                    let th = rgba.height() as usize;
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                        [tw, th],
                        rgba.as_flat_samples().as_slice(),
                    );
                    let texture = ctx.load_texture(
                        "item_preview_img",
                        color_img,
                        egui::TextureOptions::LINEAR,
                    );
                    self.preview_state = PreviewState::Image {
                        name,
                        texture,
                        orig_w,
                        orig_h,
                        exif_info,
                        tex_size: egui::vec2(tw as f32, th as f32),
                    };
                }
                Err(e) => {
                    self.preview_state = PreviewState::BinaryOrOther {
                        name,
                        info: format!("Ошибка чтения изображения: {}", e),
                    };
                }
            }
            return;
        }

        match fs::File::open(&path) {
            Ok(file) => {
                let mut buf = Vec::with_capacity(4096);
                let mut chunk = file.take(4096);
                if chunk.read_to_end(&mut buf).is_ok() {
                    if buf.is_empty() {
                        self.preview_state = PreviewState::Text {
                            name,
                            content: self.lang.tr("(Пустой файл)", "(Empty file)").to_string(),
                            truncated: false,
                        };
                    } else if buf.iter().take(512).any(|&b| b == 0) {
                        self.preview_state = PreviewState::BinaryOrOther {
                            name,
                            info: self
                                .lang
                                .tr(
                                    "Бинарный файл (предпросмотр текста недоступен)",
                                    "Binary file (text preview unavailable)",
                                )
                                .to_string(),
                        };
                    } else {
                        let truncated = buf.len() == 4096;
                        let text = String::from_utf8_lossy(&buf).to_string();
                        self.preview_state = PreviewState::Text {
                            name,
                            content: text,
                            truncated,
                        };
                    }
                } else {
                    self.preview_state = PreviewState::BinaryOrOther {
                        name,
                        info: self
                            .lang
                            .tr("Не удалось прочитать файл", "Failed to read file")
                            .to_string(),
                    };
                }
            }
            Err(e) => {
                self.preview_state = PreviewState::BinaryOrOther {
                    name,
                    info: format!("Ошибка доступа: {}", e),
                };
            }
        }
    }

    fn delete_paths_to_trash(&mut self, targets: Vec<PathBuf>) {
        if targets.is_empty() {
            return;
        }
        let count = targets.len();
        match trash::delete_all(&targets) {
            Ok(()) => {
                for p in &targets {
                    self.excluded_paths.remove(p);
                    if self.selected_path.as_ref() == Some(p) {
                        self.selected_path = None;
                        self.preview_state = PreviewState::Empty;
                    }
                }
                if let Some(dir) = self.current_dir.clone() {
                    self.load_directory(dir);
                }
                self.status_msg = match self.lang {
                    Lang::Ru => format!("Перемещено в Корзину объектов: {}", count),
                    Lang::En => format!("Moved to Recycle Bin: {} items", count),
                };
            }
            Err(e) => {
                self.status_msg = match self.lang {
                    Lang::Ru => format!("Ошибка удаления в Корзину: {}", e),
                    Lang::En => format!("Failed to move to Recycle Bin: {}", e),
                };
            }
        }
    }

    fn open_new_directory(&mut self, root_dir: PathBuf) {
        if self.current_dir.as_ref() != Some(&root_dir) {
            self.excluded_paths.clear();
            self.selected_path = None;
            self.preview_state = PreviewState::Empty;
        }
        self.load_directory(root_dir);
    }

    fn load_directory(&mut self, root_dir: PathBuf) {
        let mut entries = Vec::new();
        let mut subs = Vec::new();

        if let Ok(read_top) = fs::read_dir(&root_dir) {
            for e in read_top.flatten() {
                let p = e.path();
                if p.is_dir() {
                    let meta = e.metadata().ok();
                    if self.hide_system_hidden && Self::is_entry_hidden(&p, meta.as_ref()) {
                        continue;
                    }
                    subs.push(p);
                }
            }
        }
        subs.sort_by(|a, b| {
            let na = a.file_name().unwrap_or_default().to_string_lossy();
            let nb = b.file_name().unwrap_or_default().to_string_lossy();
            natural_cmp(&na, &nb)
        });
        self.subfolders = subs;

        self.collect_entries(&root_dir, &root_dir, 0, &mut entries);

        self.current_dir = Some(root_dir);
        self.all_entries = entries;
        self.recalculate_preview();
    }

    fn collect_entries(
        &self,
        root: &Path,
        current: &Path,
        depth: usize,
        out: &mut Vec<FsEntry>,
    ) {
        let Ok(read_dir) = fs::read_dir(current) else {
            return;
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            let meta = entry.metadata().ok();
            let is_dir = meta
                .as_ref()
                .map(|m| m.is_dir())
                .unwrap_or_else(|| path.is_dir());
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified_ts = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let is_hidden = Self::is_entry_hidden(&path, meta.as_ref());
            if self.hide_system_hidden && is_hidden {
                continue;
            }

            let parent_dir = path.parent().unwrap_or(root).to_path_buf();
            let parent_name = parent_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let rel_parent = parent_dir
                .strip_prefix(root)
                .ok()
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            let rel_display = if rel_parent.is_empty() {
                ".".to_string()
            } else {
                rel_parent
            };

            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let ext_lower = if is_dir {
                String::new()
            } else {
                path.extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
            };

            let (img_dims, exif_date, exif_camera) = if !is_dir && is_image_ext(&ext_lower) {
                let dims = image::image_dimensions(&path).ok();
                let (ed, ec) = if ext_lower == "jpg" || ext_lower == "jpeg" {
                    extract_jpeg_exif(&path)
                } else {
                    (None, None)
                };
                (dims, ed, ec)
            } else {
                (None, None, None)
            };

            out.push(FsEntry {
                path: path.clone(),
                parent_dir,
                parent_name,
                rel_parent: rel_display,
                name,
                ext_lower,
                is_dir,
                is_hidden,
                depth,
                size,
                modified_ts,
                img_dims,
                exif_date,
                exif_camera,
            });

            if is_dir && self.recursive {
                self.collect_entries(root, &path, depth + 1, out);
            }
        }
    }

    fn compare_entries(&self, a: &FsEntry, b: &FsEntry) -> Ordering {
        if self.folders_first && a.is_dir != b.is_dir {
            return b.is_dir.cmp(&a.is_dir);
        }

        let base_ord = match self.sort_field {
            SortField::NaturalName => natural_cmp(&a.name, &b.name),
            SortField::AlphaName => a
                .name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.name.cmp(&b.name)),
            SortField::ModifiedDate => a
                .modified_ts
                .cmp(&b.modified_ts)
                .then_with(|| natural_cmp(&a.name, &b.name)),
            SortField::Size => a
                .size
                .cmp(&b.size)
                .then_with(|| natural_cmp(&a.name, &b.name)),
            SortField::Extension => a
                .ext_lower
                .cmp(&b.ext_lower)
                .then_with(|| natural_cmp(&a.name, &b.name)),
            SortField::FolderPath => natural_cmp(&a.rel_parent, &b.rel_parent)
                .then_with(|| natural_cmp(&a.name, &b.name)),
        };

        if self.sort_asc {
            base_ord
        } else {
            base_ord.reverse()
        }
    }

    fn toggle_column_sort(&mut self, field: SortField) {
        if self.sort_field == field {
            self.sort_asc = !self.sort_asc;
        } else {
            self.sort_field = field;
            self.sort_asc = true;
        }
        self.recalculate_preview();
    }

    /// Подстановка умных тегов {date}, {year}, {month}, {day}, {parent}, {w}x{h}, {exif_date}, {camera}
    fn expand_smart_tags(&self, template: &str, entry: &FsEntry) -> String {
        if !template.contains('{') {
            return template.to_string();
        }
        let (year, month, day, _, _) = decompose_timestamp(entry.modified_ts);
        let date_s = format!("{:04}-{:02}-{:02}", year, month, day);
        let year_s = format!("{:04}", year);
        let month_s = format!("{:02}", month);
        let day_s = format!("{:02}", day);

        let exif_d = entry.exif_date.as_deref().unwrap_or(&date_s);
        let cam_s = entry.exif_camera.as_deref().unwrap_or("");

        let (w_s, h_s, wxh_s) = if let Some((w, h)) = entry.img_dims {
            (w.to_string(), h.to_string(), format!("{}x{}", w, h))
        } else {
            (String::new(), String::new(), String::new())
        };

        template
            .replace("{date}", &date_s)
            .replace("{year}", &year_s)
            .replace("{month}", &month_s)
            .replace("{day}", &day_s)
            .replace("{exif_date}", exif_d)
            .replace("{camera}", cam_s)
            .replace("{parent}", &entry.parent_name)
            .replace("{w}x{h}", &wxh_s)
            .replace("{w}", &w_s)
            .replace("{h}", &h_s)
    }

    fn compute_new_name(
        &self,
        entry: &FsEntry,
        idx: usize,
        custom_list_override: Option<&str>,
    ) -> String {
        let (mut stem, raw_ext) = if entry.is_dir {
            (entry.name.clone(), String::new())
        } else {
            let s = entry
                .path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let e = entry
                .path
                .extension()
                .map(|ext_os| ext_os.to_string_lossy().to_string())
                .unwrap_or_default();
            (s, e)
        };

        if let Some(custom_line) = custom_list_override {
            let clean = custom_line.trim();
            if !clean.is_empty() {
                if entry.is_dir || clean.contains('.') {
                    return clean.to_string();
                } else if raw_ext.is_empty() {
                    return clean.to_string();
                } else {
                    return format!("{}.{}", clean, raw_ext);
                }
            }
        }

        let chars: Vec<char> = stem.chars().collect();
        let start = self.trim_start.min(chars.len());
        let end = chars.len().saturating_sub(self.trim_end).max(start);
        stem = chars[start..end].iter().collect();

        if !self.find_str.is_empty() {
            let pattern = if self.use_regex {
                self.find_str.clone()
            } else {
                regex::escape(&self.find_str)
            };
            if let Ok(re) = RegexBuilder::new(&pattern)
                .case_insensitive(!self.case_sensitive)
                .build()
            {
                let rep_expanded = self.expand_smart_tags(&self.replace_str, entry);
                stem = re.replace_all(&stem, rep_expanded.as_str()).to_string();
            }
        }

        if !self.insert_text.is_empty() {
            let ins_expanded = self.expand_smart_tags(&self.insert_text, entry);
            let current_chars: Vec<char> = stem.chars().collect();
            let idx_pos = if self.insert_from_end {
                current_chars.len().saturating_sub(self.insert_pos.min(current_chars.len()))
            } else {
                self.insert_pos.min(current_chars.len())
            };
            let left: String = current_chars[..idx_pos].iter().collect();
            let right: String = current_chars[idx_pos..].iter().collect();
            stem = format!("{}{}{}", left, ins_expanded, right);
        }

        let pref_exp = self.expand_smart_tags(&self.prefix, entry);
        let suff_exp = self.expand_smart_tags(&self.suffix, entry);
        stem = format!("{}{}{}", pref_exp, stem, suff_exp);

        stem = match self.translit_mode {
            TranslitMode::None => stem,
            TranslitMode::RuToLat => transliterate_ru_to_lat(&stem),
            TranslitMode::LatToRu => transliterate_lat_to_ru(&stem),
        };

        if self.sanitize_names {
            stem = sanitize_stem(&stem);
        }

        stem = match self.case_mode {
            CaseMode::None => stem,
            CaseMode::Lower => stem.to_lowercase(),
            CaseMode::Upper => stem.to_uppercase(),
            CaseMode::Capitalize => {
                let mut c = stem.chars();
                match c.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()
                    }
                }
            }
            CaseMode::Title => stem
                .split_whitespace()
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => {
                            f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        };

        if self.num_mode != NumMode::None {
            let num = self.num_start + idx * self.num_step.max(1);
            let num_str = format!("{:0width$}", num, width = self.num_pad.max(1));
            stem = match self.num_mode {
                NumMode::Suffix if !stem.is_empty() => {
                    format!("{}{}{}", stem, self.num_sep, num_str)
                }
                NumMode::Prefix if !stem.is_empty() => {
                    format!("{}{}{}", num_str, self.num_sep, stem)
                }
                _ => num_str,
            };
        }

        let final_ext = if entry.is_dir {
            String::new()
        } else {
            let custom_clean = self.new_ext.trim().trim_start_matches('.');
            let mut ext_val = if !custom_clean.is_empty() {
                custom_clean.to_string()
            } else {
                raw_ext
            };

            if self.translit_mode == TranslitMode::RuToLat {
                ext_val = transliterate_ru_to_lat(&ext_val);
            }

            ext_val = match self.ext_mode {
                ExtMode::Keep => ext_val,
                ExtMode::Lower => ext_val.to_lowercase(),
                ExtMode::Upper => ext_val.to_uppercase(),
                ExtMode::Remove => String::new(),
            };

            if ext_val.is_empty() {
                String::new()
            } else {
                format!(".{}", ext_val)
            }
        };

        format!("{}{}", stem, final_ext)
    }

    fn recalculate_preview(&mut self) {
        let allowed_exts: Vec<String> = self
            .ext_filter
            .split(',')
            .map(|s| s.trim().trim_start_matches('.').to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let excluded_exts: Vec<String> = self
            .ext_exclude_filter
            .split(',')
            .map(|s| s.trim().trim_start_matches('.').to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let excluded_words: Vec<String> = self
            .exclude_words_filter
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let search_trimmed = self.search_query.trim();
        let search_re = if !search_trimmed.is_empty() {
            let pat = if self.search_regex {
                search_trimmed.to_string()
            } else {
                regex::escape(search_trimmed)
            };
            RegexBuilder::new(&pat)
                .case_insensitive(!self.search_case_sensitive)
                .build()
                .ok()
        } else {
            None
        };

        let mut filtered: Vec<&FsEntry> = self
            .all_entries
            .iter()
            .filter(|e| {
                if self.hide_system_hidden && e.is_hidden {
                    return false;
                }
                if e.is_dir && !self.include_dirs {
                    return false;
                }
                if !e.is_dir && !self.include_files {
                    return false;
                }

                if !search_trimmed.is_empty() {
                    if let Some(re) = &search_re {
                        if !re.is_match(&e.name) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                if !excluded_words.is_empty() {
                    let name_low = e.name.to_lowercase();
                    if excluded_words.iter().any(|w| name_low.contains(w)) {
                        return false;
                    }
                }

                if !allowed_exts.is_empty() {
                    if e.is_dir || !allowed_exts.contains(&e.ext_lower) {
                        return false;
                    }
                }
                if !excluded_exts.is_empty() && !e.is_dir && excluded_exts.contains(&e.ext_lower) {
                    return false;
                }

                if !e.is_dir {
                    let min_b = self.size_min_kb.saturating_mul(1024);
                    let max_b = self.size_max_kb.saturating_mul(1024);
                    match self.size_filter_mode {
                        SizeFilterMode::Any => {}
                        SizeFilterMode::GreaterThanKB => {
                            if e.size < min_b {
                                return false;
                            }
                        }
                        SizeFilterMode::LessThanKB => {
                            if e.size > max_b {
                                return false;
                            }
                        }
                        SizeFilterMode::BetweenKB => {
                            if e.size < min_b || e.size > max_b {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .collect();

        filtered.sort_by(|a, b| self.compare_entries(a, b));

        let custom_lines: Vec<&str> = if self.use_custom_list {
            self.custom_list_raw
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        self.preview_rows.clear();
        let mut changed_count = 0;
        let mut included_count = 0;
        let mut dir_counters: std::collections::HashMap<PathBuf, usize> =
            std::collections::HashMap::new();
        let mut global_counter: usize = 0;

        for entry in filtered {
            let included = !self.excluded_paths.contains(&entry.path);
            let old_name = entry.name.clone();

            let new_name = if included {
                let list_override = if self.use_custom_list {
                    custom_lines.get(global_counter).copied()
                } else {
                    None
                };

                let idx = if self.reset_num_per_dir {
                    let c = dir_counters.entry(entry.parent_dir.clone()).or_insert(0);
                    let current = *c;
                    *c += 1;
                    current
                } else {
                    global_counter
                };
                global_counter += 1;
                self.compute_new_name(entry, idx, list_override)
            } else {
                old_name.clone()
            };

            let changed = included && old_name != new_name && !new_name.is_empty();

            let pass_status = match self.status_filter {
                StatusFilter::All => true,
                StatusFilter::OnlyChanged => changed,
                StatusFilter::OnlyIncluded => included,
                StatusFilter::OnlyUnchanged => !changed,
            };

            if !pass_status {
                continue;
            }

            if included {
                included_count += 1;
            }
            if changed {
                changed_count += 1;
            }

            self.preview_rows.push(PreviewRow {
                old_path: entry.path.clone(),
                parent_dir: entry.parent_dir.clone(),
                rel_parent: entry.rel_parent.clone(),
                is_dir: entry.is_dir,
                depth: entry.depth,
                size: entry.size,
                modified_ts: entry.modified_ts,
                old_name,
                new_name,
                included,
                changed,
            });
        }

        self.status_msg = match self.lang {
            Lang::Ru => format!(
                "Показано: {} (из {})  •  Выбрано: {}  •  К переименованию: {}",
                self.preview_rows.len(),
                self.all_entries.len(),
                included_count,
                changed_count
            ),
            Lang::En => format!(
                "Shown: {} (of {})  •  Selected: {}  •  Ready to rename: {}",
                self.preview_rows.len(),
                self.all_entries.len(),
                included_count,
                changed_count
            ),
        };
    }

    fn reset_rules(&mut self) {
        self.find_str.clear();
        self.replace_str.clear();
        self.use_regex = false;
        self.case_sensitive = true;
        self.case_mode = CaseMode::None;
        self.translit_mode = TranslitMode::None;
        self.sanitize_names = false;
        self.prefix.clear();
        self.suffix.clear();
        self.trim_start = 0;
        self.trim_end = 0;
        self.insert_text.clear();
        self.insert_pos = 0;
        self.insert_from_end = false;
        self.use_custom_list = false;
        self.num_mode = NumMode::None;
        self.num_start = 1;
        self.num_step = 1;
        self.num_pad = 2;
        self.num_sep = "_".to_string();
        self.ext_mode = ExtMode::Keep;
        self.new_ext.clear();
        self.ext_filter.clear();
        self.ext_exclude_filter.clear();
        self.exclude_words_filter.clear();
        self.size_filter_mode = SizeFilterMode::Any;
        self.status_filter = StatusFilter::All;
        self.recalculate_preview();
    }

    fn apply_rename(&mut self) {
        let mut seen: HashSet<(PathBuf, String)> = HashSet::new();
        for row in &self.preview_rows {
            let final_name = if row.included {
                &row.new_name
            } else {
                &row.old_name
            };
            let key = (row.parent_dir.clone(), final_name.to_lowercase());
            if !seen.insert(key) {
                self.status_msg = match self.lang {
                    Lang::Ru => format!(
                        "Ошибка: дубликат имени '{}' в '{}'",
                        final_name, row.rel_parent
                    ),
                    Lang::En => format!(
                        "Error: duplicate name '{}' in '{}'",
                        final_name, row.rel_parent
                    ),
                };
                return;
            }
        }

        let mut to_rename: Vec<PreviewRow> = self
            .preview_rows
            .iter()
            .filter(|r| r.included && r.changed)
            .cloned()
            .collect();

        to_rename.sort_by(|a, b| {
            b.depth
                .cmp(&a.depth)
                .then_with(|| a.is_dir.cmp(&b.is_dir))
        });

        let mut batch_history = Vec::new();
        for row in to_rename {
            let new_path = row.old_path.with_file_name(&row.new_name);
            let same_file_case_change =
                row.old_name.to_lowercase() == row.new_name.to_lowercase();

            if new_path.exists() && !same_file_case_change {
                self.status_msg = match self.lang {
                    Lang::Ru => format!("Ошибка: '{}' уже существует", row.new_name),
                    Lang::En => format!("Error: '{}' already exists", row.new_name),
                };
                break;
            }

            if let Err(e) = fs::rename(&row.old_path, &new_path) {
                self.status_msg = format!("Error {}: {}", row.old_name, e);
                break;
            }
            batch_history.push((new_path, row.old_path));
        }

        let count = batch_history.len();
        if count > 0 {
            if self.undo_stack.len() >= 20 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(batch_history);
        }

        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir);
        }
        self.status_msg = match self.lang {
            Lang::Ru => format!("Успешно переименовано объектов: {}", count),
            Lang::En => format!("Successfully renamed items: {}", count),
        };
    }

    fn undo_rename(&mut self) {
        let Some(last_batch) = self.undo_stack.pop() else {
            return;
        };
        let mut restored = 0;
        for (curr, orig) in last_batch.into_iter().rev() {
            if fs::rename(&curr, &orig).is_ok() {
                restored += 1;
            }
        }
        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir);
        }
        self.status_msg = match self.lang {
            Lang::Ru => format!(
                "Откат выполнен: возвращено {} имён (осталось шагов в истории: {})",
                restored,
                self.undo_stack.len()
            ),
            Lang::En => format!(
                "Undo complete: restored {} names ({} steps left in history)",
                restored,
                self.undo_stack.len()
            ),
        };
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) -> (bool, bool, bool) {
        let mut trig_reload = false;
        let mut trig_undo = false;
        let mut trig_apply = false;

        let wants_kb = ctx.wants_keyboard_input();

        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) {
                trig_reload = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !wants_kb {
                trig_undo = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Enter) {
                trig_apply = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.open_new_directory(folder);
                }
            }
            if i.key_pressed(egui::Key::Delete) && !wants_kb {
                let has_checked = self.preview_rows.iter().any(|r| r.included);
                if has_checked {
                    self.confirm_trash_modal = true;
                }
            }
        });

        (trig_reload, trig_undo, trig_apply)
    }

    fn handle_drag_and_drop(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            if let Some(dropped_path) = dropped_files.into_iter().find_map(|f| f.path) {
                let target_dir = if dropped_path.is_dir() {
                    Some(dropped_path)
                } else {
                    dropped_path.parent().map(|p| p.to_path_buf())
                };

                if let Some(dir) = target_dir {
                    self.open_new_directory(dir);
                }
            }
        }

        let is_hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if is_hovering {
            let lang = self.lang;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drag_and_drop_overlay"),
            ));
            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(185));
            painter.text(
                screen_rect.center(),
                egui::Align2::CENTER_CENTER,
                lang.tr(
                    "Отпустите мышь, чтобы открыть папку в ReStem",
                    "Drop folder here to open in ReStem",
                ),
                egui::FontId::proportional(24.0),
                egui::Color32::from_rgb(30, 200, 160),
            );
        }
    }
}

impl eframe::App for ReStemApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_drag_and_drop(ctx);
        let (kb_reload, kb_undo, kb_apply) = self.handle_keyboard_shortcuts(ctx);

        let lang = self.lang;
        let is_dark = self.dark_mode;
        let mut needs_recalc = false;
        let mut needs_reload = kb_reload;
        let mut navigate_to: Option<PathBuf> = None;
        let mut trigger_apply = kb_apply;
        let mut trigger_undo = kb_undo;
        let mut trigger_reset = false;
        let mut clicked_sort_col: Option<SortField> = None;
        let mut inspect_target: Option<PathBuf> = None;
        let mut open_sys_target: Option<PathBuf> = None;
        let mut single_trash_target: Option<PathBuf> = None;
        let mut settings_dirty = false;

        let checked_count = self.preview_rows.iter().filter(|r| r.included).count();

        // --- МОДАЛЬНОЕ ОКНО: ШПАРГАЛКА REGEX ---
        if self.show_regex_help_modal {
            let mut open = true;
            egui::Window::new(lang.tr("🧩 Шпаргалка по Regex", "🧩 Regex Cheat Sheet"))
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(lang.tr(
                        "Нажмите на шаблон, чтобы сразу подставить его в «Поиск и замену»:",
                        "Click a template to insert it directly into Find & Replace:",
                    ));
                    ui.add_space(6.0);

                    let templates = [
                        (
                            lang.tr("Убрать номер копии в скобках: « (1)», « (2)»", "Remove copy numbers: ' (1)', ' (2)'"),
                            r"\s*\(\d+\)",
                            "",
                        ),
                        (
                            lang.tr("Убрать текст в квадратных скобках [тег]", "Remove bracketed tags [tag]"),
                            r"\s*\[[^\]]*\]",
                            "",
                        ),
                        (
                            lang.tr("Переставить дату ДД.ММ.ГГГГ → ГГГГ-ММ-ДД", "Swap date DD.MM.YYYY → YYYY-MM-DD"),
                            r"(\d{2})\.(\d{2})\.(\d{4})",
                            "$3-$2-$1",
                        ),
                        (
                            lang.tr("Удалить все цифры из имени", "Remove all digits from name"),
                            r"\d+",
                            "",
                        ),
                        (
                            lang.tr("Оставить только цифры", "Keep digits only"),
                            r"[^\d]+",
                            "",
                        ),
                        (
                            lang.tr("Поменять местами 2 слова через дефис (А-Б → Б-А)", "Swap two hyphenated words (A-B → B-A)"),
                            r"^([^-]+)-(.+)$",
                            "$2-$1",
                        ),
                    ];

                    for (title, f_pat, r_pat) in templates {
                        if ui.button(format!("🔹 {}", title)).clicked() {
                            self.find_str = f_pat.to_string();
                            self.replace_str = r_pat.to_string();
                            self.use_regex = true;
                            needs_recalc = true;
                            self.show_regex_help_modal = false;
                        }
                    }

                    ui.separator();
                    ui.weak(lang.tr(
                        "Обозначения: \\d — цифра, \\s — пробел, .+ — любые символы, $1 / $2 — найденные группы в скобках.",
                        "Tokens: \\d = digit, \\s = space, .+ = any chars, $1 / $2 = capture groups in parentheses.",
                    ));
                });
            if !open {
                self.show_regex_help_modal = false;
            }
        }

        // --- МОДАЛЬНОЕ ОКНО: ПЕРЕИМЕНОВАНИЕ ПО СПИСКУ (.TXT / .CSV) ---
        if self.show_list_rename_modal {
            let mut open = true;
            egui::Window::new(lang.tr("📋 Переименование по списку", "📋 Rename from List"))
                .collapsible(false)
                .resizable(true)
                .default_width(450.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    needs_recalc |= ui
                        .checkbox(
                            &mut self.use_custom_list,
                            lang.tr(
                                "Включить переименование по списку (1 строка = 1 файл)",
                                "Enable list renaming (1 line = 1 file)",
                            ),
                        )
                        .changed();

                    ui.horizontal(|ui| {
                        if ui
                            .button(lang.tr("📂 Загрузить из .txt / .csv", "📂 Load from .txt / .csv"))
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Text / CSV", &["txt", "csv"])
                                .pick_file()
                            {
                                if let Ok(txt) = fs::read_to_string(path) {
                                    self.custom_list_raw = txt;
                                    self.use_custom_list = true;
                                    needs_recalc = true;
                                }
                            }
                        }
                        if ui.button(lang.tr("🧹 Очистить список", "🧹 Clear list")).clicked() {
                            self.custom_list_raw.clear();
                            needs_recalc = true;
                        }
                    });

                    ui.label(lang.tr(
                        "Вставьте список новых имён (по одному на строку):",
                        "Paste new filenames (one per line):",
                    ));
                    if ui
                        .add(
                            egui::TextEdit::multiline(&mut self.custom_list_raw)
                                .desired_rows(10)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        needs_recalc = true;
                    }
                });
            if !open {
                self.show_list_rename_modal = false;
            }
        }

        // --- МОДАЛЬНОЕ ОКНО «О ПРОГРАММЕ» + GITHUB И КОНТЕКСТНОЕ МЕНЮ ---
        if self.show_about_modal {
            let mut open = true;
            let mut close_about = false;
            let mut reg_action: Option<bool> = None;

            egui::Window::new(lang.tr("ℹ О программе ReStem", "ℹ About ReStem"))
                .collapsible(false)
                .resizable(false)
                .default_width(425.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 10.0, egui::Color32::from_rgb(20, 30, 48));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "R",
                            egui::FontId::proportional(28.0),
                            egui::Color32::from_rgb(35, 205, 175),
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("ReStem").strong().size(20.0));
                            ui.label(lang.tr(
                                "Пакетное переименование файлов и папок",
                                "Fast Bulk File & Folder Renamer",
                            ));
                            ui.colored_label(
                                egui::Color32::from_rgb(24, 150, 105),
                                format!(
                                    "{}: v{} (Windows x64 Portable)",
                                    lang.tr("Версия", "Version"),
                                    APP_VERSION
                                ),
                            );
                        });
                    });

                    ui.add_space(8.0);
                    card_frame(is_dark).show(ui, |ui| {
                        ui.set_min_width(385.0);
                        ui.label(lang.tr(
                            "• Рекурсивное переименование файлов и каталогов\n\
                             • Умные теги {date}, {exif_date}, {camera}, {parent}, {w}x{h}\n\
                             • Посимвольная Diff-подсветка, Regex и транслит (RU ↔ LAT)\n\
                             • Предпросмотр картинок и текста, удаление в Корзину\n\
                             • Многошаговый откат (Ctrl+Z) и экспорт скрипта .bat",
                            "• Recursive file and folder batch renaming\n\
                             • Smart tags {date}, {exif_date}, {camera}, {parent}, {w}x{h}\n\
                             • Inline character Diff, Regex & Transliteration (RU ↔ LAT)\n\
                             • Image/text preview & safe Recycle Bin deletion\n\
                             • Multi-step Undo (Ctrl+Z) & .bat rollback export",
                        ));

                        ui.separator();
                        ui.strong(lang.tr(
                            "Официальный репозиторий GitHub:",
                            "Official GitHub Repository:",
                        ));
                        ui.weak(GITHUB_REPO_URL);
                        ui.horizontal(|ui| {
                            if ui
                                .button(lang.tr(
                                    "🌐 Открыть GitHub",
                                    "🌐 Open GitHub Repo",
                                ))
                                .clicked()
                            {
                                open_url_in_browser(GITHUB_REPO_URL);
                            }
                            if ui
                                .button(lang.tr(
                                    "⬇ Проверить обновления (Releases)",
                                    "⬇ Check Releases",
                                ))
                                .clicked()
                            {
                                let rel_url = format!(
                                    "{}/releases/latest",
                                    GITHUB_REPO_URL.trim_end_matches('/')
                                );
                                open_url_in_browser(&rel_url);
                            }
                        });

                        ui.separator();
                        ui.strong(lang.tr(
                            "Интеграция в контекстное меню Проводника Windows:",
                            "Windows Explorer Context Menu Integration:",
                        ));
                        ui.horizontal(|ui| {
                            if ui
                                .button(lang.tr(
                                    "➕ Добавить «Открыть в ReStem»",
                                    "➕ Add 'Open in ReStem'",
                                ))
                                .clicked()
                            {
                                reg_action = Some(true);
                            }
                            if ui
                                .button(lang.tr("✖ Удалить из меню", "✖ Remove from menu"))
                                .clicked()
                            {
                                reg_action = Some(false);
                            }
                        });
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.weak(lang.tr("Горячие клавиши:", "Shortcuts:"));
                            ui.label("Ctrl+O • F5 • Ctrl+Z • Del • Ctrl+Enter");
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(lang.tr("📋 Скопировать версию", "📋 Copy version info"))
                            .clicked()
                        {
                            ctx.copy_text(format!(
                                "ReStem v{} — {}",
                                APP_VERSION, GITHUB_REPO_URL
                            ));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(lang.tr("Закрыть", "Close")).clicked() {
                                close_about = true;
                            }
                        });
                    });
                });

            if let Some(install) = reg_action {
                self.register_explorer_context_menu(install);
            }
            if close_about || !open {
                self.show_about_modal = false;
            }
        }

        // --- МОДАЛЬНОЕ ОКНО ПОДТВЕРЖДЕНИЯ УДАЛЕНИЯ В КОРЗИНУ ---
        if self.confirm_trash_modal {
            let mut open = true;
            let mut confirm_now = false;
            let mut close_now = false;

            egui::Window::new(lang.tr("🗑 Удаление в Корзину", "🗑 Move to Recycle Bin"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    let msg = match lang {
                        Lang::Ru => format!(
                            "Переместить отмеченные галочками объекты ({} шт.) в Корзину Windows?",
                            checked_count
                        ),
                        Lang::En => format!(
                            "Move checked items ({} total) to the Windows Recycle Bin?",
                            checked_count
                        ),
                    };
                    ui.label(msg);
                    ui.weak(lang.tr(
                        "Вы сможете восстановить их из стандартной Корзины Windows.",
                        "You can restore them anytime from the Windows Recycle Bin.",
                    ));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let del_btn = egui::Button::new(
                            egui::RichText::new(lang.tr(
                                "🗑 Да, переместить в Корзину",
                                "🗑 Yes, Move to Recycle Bin",
                            ))
                            .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(205, 55, 55));

                        if ui.add(del_btn).clicked() {
                            confirm_now = true;
                        }
                        if ui.button(lang.tr("Отмена", "Cancel")).clicked() {
                            close_now = true;
                        }
                    });
                });

            if confirm_now {
                let targets: Vec<PathBuf> = self
                    .preview_rows
                    .iter()
                    .filter(|r| r.included)
                    .map(|r| r.old_path.clone())
                    .collect();
                self.confirm_trash_modal = false;
                self.delete_paths_to_trash(targets);
            } else if close_now || !open {
                self.confirm_trash_modal = false;
            }
        }

        // --- ВЕРХНЯЯ ПАНЕЛЬ ИНСТРУМЕНТОВ ---
        egui::TopBottomPanel::top("top_toolbar").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 6.0, egui::Color32::from_rgb(20, 30, 48));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "R",
                    egui::FontId::proportional(17.0),
                    egui::Color32::from_rgb(35, 205, 175),
                );
                ui.strong(egui::RichText::new("ReStem").size(17.0));
                ui.weak(format!("v{}", APP_VERSION));
                ui.separator();

                if ui
                    .button(lang.tr("📁 Папка...", "📁 Folder..."))
                    .on_hover_text("Ctrl+O")
                    .clicked()
                {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        navigate_to = Some(folder);
                    }
                }

                if ui
                    .button(lang.tr("🔄 Обновить", "🔄 Refresh"))
                    .on_hover_text("F5")
                    .clicked()
                {
                    needs_reload = true;
                }

                if ui
                    .button(lang.tr("☑ Инверт.", "☑ Invert"))
                    .clicked()
                {
                    for row in &self.preview_rows {
                        if self.excluded_paths.contains(&row.old_path) {
                            self.excluded_paths.remove(&row.old_path);
                        } else {
                            self.excluded_paths.insert(row.old_path.clone());
                        }
                    }
                    needs_recalc = true;
                }

                let undo_label = match lang {
                    Lang::Ru => format!("↺ Откат ({})", self.undo_stack.len()),
                    Lang::En => format!("↺ Undo ({})", self.undo_stack.len()),
                };
                if ui
                    .add_enabled(!self.undo_stack.is_empty(), egui::Button::new(undo_label))
                    .on_hover_text("Ctrl+Z")
                    .clicked()
                {
                    trigger_undo = true;
                }

                if ui
                    .add_enabled(
                        !self.undo_stack.is_empty(),
                        egui::Button::new(lang.tr("💾 .bat", "💾 .bat")),
                    )
                    .on_hover_text(lang.tr(
                        "Экспортировать .bat-скрипт для отката последнего переименования",
                        "Export .bat rollback script for the last batch rename",
                    ))
                    .clicked()
                {
                    self.export_rollback_script();
                }

                let trash_label = match lang {
                    Lang::Ru => format!("🗑 В корзину ({})", checked_count),
                    Lang::En => format!("🗑 Trash ({})", checked_count),
                };
                if ui
                    .add_enabled(checked_count > 0, egui::Button::new(trash_label))
                    .on_hover_text("Delete")
                    .clicked()
                {
                    self.confirm_trash_modal = true;
                }

                ui.separator();

                ui.label("🔍");
                let search_hint = lang.tr("Поиск...", "Filter...");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(search_hint)
                            .desired_width(115.0),
                    )
                    .changed()
                {
                    needs_recalc = true;
                }
                if ui
                    .selectable_label(self.search_case_sensitive, "Aa")
                    .clicked()
                {
                    self.search_case_sensitive = !self.search_case_sensitive;
                    needs_recalc = true;
                }
                if ui.selectable_label(self.search_regex, ".*").clicked() {
                    self.search_regex = !self.search_regex;
                    needs_recalc = true;
                }
                if !self.search_query.is_empty() && ui.small_button("✖").clicked() {
                    self.search_query.clear();
                    needs_recalc = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(lang.tr("ℹ О программе", "ℹ About"))
                        .clicked()
                    {
                        self.show_about_modal = true;
                    }

                    if ui
                        .button("🌐 GitHub")
                        .on_hover_text(GITHUB_REPO_URL)
                        .clicked()
                    {
                        open_url_in_browser(GITHUB_REPO_URL);
                    }

                    ui.separator();

                    if ui.selectable_label(self.lang == Lang::En, "EN").clicked() {
                        self.lang = Lang::En;
                        settings_dirty = true;
                        needs_recalc = true;
                    }
                    if ui.selectable_label(self.lang == Lang::Ru, "RU").clicked() {
                        self.lang = Lang::Ru;
                        settings_dirty = true;
                        needs_recalc = true;
                    }

                    ui.separator();

                    if ui
                        .small_button("A+")
                        .on_hover_text(lang.tr("Увеличить масштаб", "Zoom In"))
                        .clicked()
                    {
                        self.ui_scale = (self.ui_scale + 0.1).min(1.5);
                        ctx.set_pixels_per_point(self.ui_scale);
                        settings_dirty = true;
                    }
                    if ui
                        .small_button(format!("{:.0}%", self.ui_scale * 100.0))
                        .on_hover_text(lang.tr("Сбросить масштаб на 100%", "Reset scale to 100%"))
                        .clicked()
                    {
                        self.ui_scale = 1.0;
                        ctx.set_pixels_per_point(self.ui_scale);
                        settings_dirty = true;
                    }
                    if ui
                        .small_button("A-")
                        .on_hover_text(lang.tr("Уменьшить масштаб", "Zoom Out"))
                        .clicked()
                    {
                        self.ui_scale = (self.ui_scale - 0.1).max(0.85);
                        ctx.set_pixels_per_point(self.ui_scale);
                        settings_dirty = true;
                    }

                    ui.separator();

                    let theme_label = if self.dark_mode {
                        lang.tr("🌙 Тёмная", "🌙 Dark")
                    } else {
                        lang.tr("☀ Светлая", "☀ Light")
                    };
                    if ui.button(theme_label).clicked() {
                        self.dark_mode = !self.dark_mode;
                        apply_theme_and_style(ctx, self.dark_mode);
                        settings_dirty = true;
                    }
                });
            });

            ui.separator();

            // Ряд 2: Панель фильтрации, Diff-режима и сортировки
            ui.horizontal(|ui| {
                needs_recalc |= ui
                    .checkbox(&mut self.include_files, lang.tr("Файлы", "Files"))
                    .changed();
                needs_recalc |= ui
                    .checkbox(&mut self.include_dirs, lang.tr("Папки", "Folders"))
                    .changed();
                if ui
                    .checkbox(&mut self.recursive, lang.tr("Вложенные", "Recursive"))
                    .changed()
                {
                    needs_reload = true;
                }
                if ui
                    .checkbox(
                        &mut self.hide_system_hidden,
                        lang.tr("Скрыть .git/сист.", "Hide .git/sys"),
                    )
                    .changed()
                {
                    needs_reload = true;
                }

                ui.separator();

                if ui
                    .checkbox(
                        &mut self.inline_diff,
                        lang.tr("🎨 Diff-подсветка букв", "🎨 Inline Diff"),
                    )
                    .changed()
                {
                    settings_dirty = true;
                }

                ui.separator();

                ui.label(lang.tr("Показать:", "Show:"));
                egui::ComboBox::from_id_salt("status_filter_cb")
                    .width(145.0)
                    .selected_text(match self.status_filter {
                        StatusFilter::All => lang.tr("Все объекты", "All items"),
                        StatusFilter::OnlyChanged => lang.tr("Только изменяемые", "Only changing"),
                        StatusFilter::OnlyIncluded => lang.tr("Только выбранные ☑", "Only checked ☑"),
                        StatusFilter::OnlyUnchanged => lang.tr("Без изменений", "Unchanged only"),
                    })
                    .show_ui(ui, |ui| {
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.status_filter,
                                StatusFilter::All,
                                lang.tr("Все объекты", "All items"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.status_filter,
                                StatusFilter::OnlyChanged,
                                lang.tr("Только изменяемые", "Only changing"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.status_filter,
                                StatusFilter::OnlyIncluded,
                                lang.tr("Только выбранные ☑", "Only checked ☑"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.status_filter,
                                StatusFilter::OnlyUnchanged,
                                lang.tr("Без изменений", "Unchanged only"),
                            )
                            .changed();
                    });

                ui.separator();

                ui.strong(lang.tr("Сортировка:", "Sort:"));
                egui::ComboBox::from_id_salt("sort_field_cb")
                    .width(175.0)
                    .selected_text(match self.sort_field {
                        SortField::NaturalName => {
                            lang.tr("Умная по имени (1, 2, 10)", "Natural Name (1, 2, 10)")
                        }
                        SortField::AlphaName => lang.tr("По алфавиту (А–Я)", "Alphabetical (A–Z)"),
                        SortField::ModifiedDate => lang.tr("По дате изменения", "Modified Date"),
                        SortField::Size => lang.tr("По размеру файла", "File Size"),
                        SortField::Extension => lang.tr("По расширению", "Extension / Type"),
                        SortField::FolderPath => lang.tr("По расположению", "Folder Path"),
                    })
                    .show_ui(ui, |ui| {
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.sort_field,
                                SortField::NaturalName,
                                lang.tr("Умная по имени (1, 2, 10)", "Natural Name (1, 2, 10)"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.sort_field,
                                SortField::AlphaName,
                                lang.tr("По алфавиту (А–Я)", "Alphabetical (A–Z)"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.sort_field,
                                SortField::ModifiedDate,
                                lang.tr("По дате изменения", "Modified Date"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.sort_field,
                                SortField::Size,
                                lang.tr("По размеру файла", "File Size"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.sort_field,
                                SortField::Extension,
                                lang.tr("По расширению", "Extension / Type"),
                            )
                            .changed();
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.sort_field,
                                SortField::FolderPath,
                                lang.tr("По расположению", "Folder Path"),
                            )
                            .changed();
                    });

                let dir_btn_label = if self.sort_asc {
                    lang.tr("▲ По возр.", "▲ Asc")
                } else {
                    lang.tr("▼ По убыв.", "▼ Desc")
                };
                if ui.button(dir_btn_label).clicked() {
                    self.sort_asc = !self.sort_asc;
                    needs_recalc = true;
                }

                needs_recalc |= ui
                    .checkbox(
                        &mut self.folders_first,
                        lang.tr("Папки сверху", "Folders first"),
                    )
                    .changed();
            });
            ui.add_space(3.0);
        });

        // --- НИЖНЯЯ СТРОКА СОСТОЯНИЯ ---
        egui::TopBottomPanel::bottom("status_footer").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.strong(&self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let path_str = self
                        .current_dir
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| {
                            lang.tr("Папка не выбрана", "No folder selected").to_string()
                        });
                    ui.label(path_str);
                });
            });
            ui.add_space(2.0);
        });

        // --- ЛЕВАЯ КОЛОНКА: ПАПКИ, ПРЕДПРОСМОТР И СОХРАНЯЕМЫЕ ПРЕСЕТЫ ---
        egui::SidePanel::left("left_explorer_panel")
            .resizable(true)
            .default_width(275.0)
            .width_range(225.0..=395.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("left_panel_main_scroll")
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr("📂 Навигация по папкам", "📂 Folder Explorer"),
                                is_dark,
                            );
                            ui.add_space(4.0);

                            if let Some(curr) = &self.current_dir {
                                if let Some(parent) = curr.parent() {
                                    if ui
                                        .button(lang.tr(
                                            "⬆ На уровень вверх (..)",
                                            "⬆ Up one level (..)",
                                        ))
                                        .clicked()
                                    {
                                        navigate_to = Some(parent.to_path_buf());
                                    }
                                }
                                let curr_name = curr
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| curr.display().to_string());
                                ui.strong(format!("📁 {}", curr_name));
                            } else {
                                ui.label(lang.tr(
                                    "Нажмите «Папка...» или перетащите каталог в окно.",
                                    "Click 'Folder...' or drag a folder into the window.",
                                ));
                            }

                            egui::ScrollArea::vertical()
                                .id_salt("folders_sub_scroll")
                                .max_height(135.0)
                                .show(ui, |ui| {
                                    for sub in &self.subfolders {
                                        let name = sub
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string();
                                        if ui
                                            .selectable_label(false, format!("  📁 {}", name))
                                            .clicked()
                                        {
                                            navigate_to = Some(sub.clone());
                                        }
                                    }
                                });
                        });

                        ui.add_space(8.0);

                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr("👁 Предпросмотр объекта", "👁 Item Preview"),
                                is_dark,
                            );
                            ui.add_space(4.0);

                            match &self.preview_state {
                                PreviewState::Empty => {
                                    ui.label(lang.tr(
                                        "Кликните по имени в таблице для превью (двойной клик — открыть файл).",
                                        "Click an item to preview (double-click to open file).",
                                    ));
                                }
                                PreviewState::Folder { name, items_count } => {
                                    ui.strong(format!("📁 {}", name));
                                    ui.label(match lang {
                                        Lang::Ru => format!("Объектов внутри: {}", items_count),
                                        Lang::En => format!("Items inside: {}", items_count),
                                    });
                                    if let Some(sel) = &self.selected_path {
                                        if ui
                                            .button(lang.tr(
                                                "📂 Войти в эту папку",
                                                "📂 Open this folder",
                                            ))
                                            .clicked()
                                        {
                                            navigate_to = Some(sel.clone());
                                        }
                                    }
                                }
                                PreviewState::Image {
                                    name,
                                    texture,
                                    orig_w,
                                    orig_h,
                                    exif_info,
                                    tex_size,
                                } => {
                                    ui.strong(format!("🖼 {}", name));
                                    ui.label(format!("{} × {} px", orig_w, orig_h));
                                    if !exif_info.is_empty() {
                                        ui.weak(exif_info);
                                    }
                                    let max_w = ui.available_width().max(80.0);
                                    let max_h = 175.0;
                                    let scale =
                                        (max_w / tex_size.x).min(max_h / tex_size.y).min(1.0);
                                    let draw_size = *tex_size * scale;
                                    let (rect, _) =
                                        ui.allocate_exact_size(draw_size, egui::Sense::hover());
                                    ui.painter().image(
                                        texture.id(),
                                        rect,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        egui::Color32::WHITE,
                                    );
                                }
                                PreviewState::Text {
                                    name,
                                    content,
                                    truncated,
                                } => {
                                    ui.strong(format!("📄 {}", name));
                                    if *truncated {
                                        ui.weak(lang.tr(
                                            "Первые 4 КБ текста:",
                                            "First 4 KB of text:",
                                        ));
                                    }
                                    egui::ScrollArea::vertical()
                                        .id_salt("text_preview_scroll")
                                        .max_height(150.0)
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(content)
                                                        .monospace()
                                                        .size(12.5),
                                                )
                                                .wrap(),
                                            );
                                        });
                                }
                                PreviewState::BinaryOrOther { name, info } => {
                                    ui.strong(format!("📄 {}", name));
                                    ui.label(info);
                                }
                            }

                            if let Some(sel_path) = &self.selected_path {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(lang.tr("🚀 Открыть", "🚀 Open"))
                                        .clicked()
                                    {
                                        open_sys_target = Some(sel_path.clone());
                                    }
                                    if ui
                                        .button(lang.tr("🗑 В Корзину", "🗑 Trash"))
                                        .clicked()
                                    {
                                        single_trash_target = Some(sel_path.clone());
                                    }
                                });
                            }
                        });

                        ui.add_space(8.0);

                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr("⚡ Шаблоны и свои пресеты", "⚡ Quick & Custom Presets"),
                                is_dark,
                            );
                            ui.add_space(4.0);

                            if ui
                                .button(lang.tr(
                                    "• Транслит (RU → LAT) + чистка",
                                    "• Translit (RU → LAT) + sanitize",
                                ))
                                .clicked()
                            {
                                self.translit_mode = TranslitMode::RuToLat;
                                self.sanitize_names = true;
                                self.find_str = " ".to_string();
                                self.replace_str = "_".to_string();
                                self.use_regex = false;
                                needs_recalc = true;
                            }
                            if ui
                                .button(lang.tr(
                                    "• Префикс даты {date}_ в начало",
                                    "• Prepend {date}_ prefix",
                                ))
                                .clicked()
                            {
                                self.prefix = "{date}_".to_string();
                                needs_recalc = true;
                            }
                            if ui
                                .button(lang.tr(
                                    "• Имя папки {parent}_ + счётчик",
                                    "• Folder {parent}_ + counter",
                                ))
                                .clicked()
                            {
                                self.prefix = "{parent}_".to_string();
                                self.num_mode = NumMode::Suffix;
                                self.num_start = 1;
                                self.num_pad = 3;
                                needs_recalc = true;
                            }
                            if ui
                                .button(lang.tr(
                                    "• Добавить разрешение _{w}x{h}",
                                    "• Append resolution _{w}x{h}",
                                ))
                                .clicked()
                            {
                                self.suffix = "_{w}x{h}".to_string();
                                needs_recalc = true;
                            }

                            if !self.user_presets.is_empty() {
                                ui.separator();
                                ui.weak(lang.tr("Мои сохранённые пресеты:", "My saved presets:"));
                                let mut remove_idx: Option<usize> = None;
                                let mut apply_idx: Option<usize> = None;

                                for (idx, p) in self.user_presets.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        if ui.button(format!("⭐ {}", p.title)).clicked() {
                                            apply_idx = Some(idx);
                                        }
                                        if ui.small_button("✖").clicked() {
                                            remove_idx = Some(idx);
                                        }
                                    });
                                }

                                if let Some(idx) = apply_idx {
                                    let p = &self.user_presets[idx];
                                    self.find_str = p.find_str.clone();
                                    self.replace_str = p.replace_str.clone();
                                    self.use_regex = p.use_regex;
                                    self.prefix = p.prefix.clone();
                                    self.suffix = p.suffix.clone();
                                    self.translit_mode = p.translit_mode;
                                    self.case_mode = p.case_mode;
                                    self.sanitize_names = p.sanitize;
                                    needs_recalc = true;
                                }
                                if let Some(idx) = remove_idx {
                                    self.user_presets.remove(idx);
                                    settings_dirty = true;
                                }
                            }

                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_preset_title)
                                        .hint_text(lang.tr("Имя пресета...", "Preset name..."))
                                        .desired_width(145.0),
                                );
                                if ui
                                    .button(lang.tr("➕ Сохранить", "➕ Save"))
                                    .clicked()
                                    && !self.new_preset_title.trim().is_empty()
                                {
                                    self.user_presets.push(UserPreset {
                                        title: self.new_preset_title.trim().to_string(),
                                        find_str: self.find_str.clone(),
                                        replace_str: self.replace_str.clone(),
                                        use_regex: self.use_regex,
                                        prefix: self.prefix.clone(),
                                        suffix: self.suffix.clone(),
                                        translit_mode: self.translit_mode,
                                        case_mode: self.case_mode,
                                        sanitize: self.sanitize_names,
                                    });
                                    self.new_preset_title.clear();
                                    settings_dirty = true;
                                }
                            });

                            ui.add_space(4.0);
                            if ui
                                .button(lang.tr(
                                    "✖ Сбросить все правила и фильтры",
                                    "✖ Reset All Rules & Filters",
                                ))
                                .clicked()
                            {
                                trigger_reset = true;
                            }
                        });
                    });
            });

        // --- ПРАВАЯ КОЛОНКА: КАРТОЧКИ ПРАВИЛ И ФИЛЬТРОВ ---
        egui::SidePanel::right("right_rules_panel")
            .resizable(true)
            .default_width(335.0)
            .width_range(290.0..=450.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.heading(lang.tr("Правила", "Rename Rules"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let list_btn_txt = if self.use_custom_list {
                            lang.tr("📋 По списку (ВКЛ)", "📋 From List (ON)")
                        } else {
                            lang.tr("📋 По списку...", "📋 From List...")
                        };
                        if ui.button(list_btn_txt).clicked() {
                            self.show_list_rename_modal = true;
                        }
                    });
                });
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("rules_scroll")
                    .show(ui, |ui| {
                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                section_title(
                                    ui,
                                    lang.tr("1. Поиск и замена", "1. Find & Replace"),
                                    is_dark,
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button(lang.tr("[?] Regex", "[?] Regex"))
                                            .clicked()
                                        {
                                            self.show_regex_help_modal = true;
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(lang.tr("Найти текст:", "Find text:"));
                            needs_recalc |= ui.text_edit_singleline(&mut self.find_str).changed();
                            ui.label(lang.tr("Заменить на:", "Replace with:"));
                            needs_recalc |=
                                ui.text_edit_singleline(&mut self.replace_str).changed();
                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.use_regex,
                                    lang.tr(
                                        "Регулярные выражения (Regex)",
                                        "Use Regular Expressions",
                                    ),
                                )
                                .changed();
                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.case_sensitive,
                                    lang.tr("Учитывать регистр", "Case Sensitive"),
                                )
                                .changed();
                        });

                        ui.add_space(8.0);

                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr(
                                    "2. Регистр, транслит и чистка",
                                    "2. Case, Translit & Clean",
                                ),
                                is_dark,
                            );
                            ui.add_space(4.0);
                            egui::ComboBox::from_id_salt("case_cb")
                                .width(265.0)
                                .selected_text(match self.case_mode {
                                    CaseMode::None => {
                                        lang.tr("Не менять регистр", "Default (No change)")
                                    }
                                    CaseMode::Lower => lang.tr("все строчные", "lowercase"),
                                    CaseMode::Upper => lang.tr("ВСЕ ПРОПИСНЫЕ", "UPPERCASE"),
                                    CaseMode::Capitalize => {
                                        lang.tr("Первая буква заглавная", "Capitalize first letter")
                                    }
                                    CaseMode::Title => {
                                        lang.tr("Каждое Слово С Заглавной", "Title Case Words")
                                    }
                                })
                                .show_ui(ui, |ui| {
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.case_mode,
                                            CaseMode::None,
                                            lang.tr("Не менять регистр", "Default (No change)"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.case_mode,
                                            CaseMode::Lower,
                                            lang.tr("все строчные", "lowercase"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.case_mode,
                                            CaseMode::Upper,
                                            lang.tr("ВСЕ ПРОПИСНЫЕ", "UPPERCASE"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.case_mode,
                                            CaseMode::Capitalize,
                                            lang.tr(
                                                "Первая буква заглавная",
                                                "Capitalize first letter",
                                            ),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.case_mode,
                                            CaseMode::Title,
                                            lang.tr("Каждое Слово С Заглавной", "Title Case Words"),
                                        )
                                        .changed();
                                });

                            ui.add_space(4.0);
                            ui.label(lang.tr("Транслитерация:", "Transliteration:"));
                            egui::ComboBox::from_id_salt("translit_cb")
                                .width(265.0)
                                .selected_text(match self.translit_mode {
                                    TranslitMode::None => lang.tr("Выключена", "Disabled"),
                                    TranslitMode::RuToLat => {
                                        lang.tr("Кириллица → Латиница (RU → LAT)", "Cyrillic → Latin (RU → LAT)")
                                    }
                                    TranslitMode::LatToRu => {
                                        lang.tr("Латиница → Кириллица (LAT → RU)", "Latin → Cyrillic (LAT → RU)")
                                    }
                                })
                                .show_ui(ui, |ui| {
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.translit_mode,
                                            TranslitMode::None,
                                            lang.tr("Выключена", "Disabled"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.translit_mode,
                                            TranslitMode::RuToLat,
                                            lang.tr("Кириллица → Латиница (RU → LAT)", "Cyrillic → Latin (RU → LAT)"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.translit_mode,
                                            TranslitMode::LatToRu,
                                            lang.tr("Латиница → Кириллица (LAT → RU)", "Latin → Cyrillic (LAT → RU)"),
                                        )
                                        .changed();
                                });

                            ui.add_space(4.0);
                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.sanitize_names,
                                    lang.tr(
                                        "Чистка имён (двойные пробелы и спецсимволы)",
                                        "Sanitize names (collapse spaces & bad chars)",
                                    ),
                                )
                                .changed();
                        });

                        ui.add_space(8.0);

                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr(
                                    "3. Вставка, обрезка и Умные теги",
                                    "3. Add, Trim & Smart Tags",
                                ),
                                is_dark,
                            );
                            ui.add_space(4.0);
                            egui::Grid::new("prefix_grid").num_columns(2).show(ui, |ui| {
                                ui.label(lang.tr("Префикс:", "Prefix:"));
                                needs_recalc |= ui.text_edit_singleline(&mut self.prefix).changed();
                                ui.end_row();

                                ui.label(lang.tr("Суффикс:", "Suffix:"));
                                needs_recalc |= ui.text_edit_singleline(&mut self.suffix).changed();
                                ui.end_row();

                                ui.label(lang.tr("Вставить текст:", "Insert text:"));
                                needs_recalc |=
                                    ui.text_edit_singleline(&mut self.insert_text).changed();
                                ui.end_row();

                                ui.label(lang.tr("На позицию:", "At index:"));
                                ui.horizontal(|ui| {
                                    needs_recalc |= ui
                                        .add(
                                            egui::DragValue::new(&mut self.insert_pos)
                                                .range(0..=200),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .checkbox(
                                            &mut self.insert_from_end,
                                            lang.tr("с конца", "from end"),
                                        )
                                        .changed();
                                });
                                ui.end_row();

                                ui.label(lang.tr("Срезать слева:", "Trim left:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.trim_start).range(0..=100))
                                    .changed();
                                ui.end_row();

                                ui.label(lang.tr("Срезать справа:", "Trim right:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.trim_end).range(0..=100))
                                    .changed();
                                ui.end_row();
                            });

                            ui.add_space(4.0);
                            ui.weak(lang.tr(
                                "Быстрая вставка тега в Префикс:",
                                "Quick insert tag into Prefix:",
                            ));
                            ui.horizontal_wrapped(|ui| {
                                for (tag, tip) in [
                                    ("{date}_", "Дата изменения (ГГГГ-ММ-ДД)"),
                                    ("{exif_date}_", "Дата съёмки фото из EXIF"),
                                    ("{camera}_", "Модель камеры из EXIF"),
                                    ("{parent}_", "Имя родительской папки"),
                                    ("{w}x{h}_", "Разрешение изображения (ШxВ)"),
                                ] {
                                    if ui.small_button(tag).on_hover_text(tip).clicked() {
                                        self.prefix.push_str(tag);
                                        needs_recalc = true;
                                    }
                                }
                            });
                        });

                        ui.add_space(8.0);

                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr(
                                    "4. Нумерация (по сортировке)",
                                    "4. Numbering (follows sort)",
                                ),
                                is_dark,
                            );
                            ui.add_space(4.0);
                            egui::ComboBox::from_id_salt("num_cb")
                                .width(265.0)
                                .selected_text(match self.num_mode {
                                    NumMode::None => lang.tr("Выключена", "Disabled"),
                                    NumMode::Suffix => {
                                        lang.tr("В конец имени (Суффикс)", "Append to end (Suffix)")
                                    }
                                    NumMode::Prefix => lang.tr(
                                        "В начало имени (Префикс)",
                                        "Prepend to start (Prefix)",
                                    ),
                                })
                                .show_ui(ui, |ui| {
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.num_mode,
                                            NumMode::None,
                                            lang.tr("Выключена", "Disabled"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.num_mode,
                                            NumMode::Suffix,
                                            lang.tr(
                                                "В конец имени (Суффикс)",
                                                "Append to end (Suffix)",
                                            ),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.num_mode,
                                            NumMode::Prefix,
                                            lang.tr(
                                                "В начало имени (Префикс)",
                                                "Prepend to start (Prefix)",
                                            ),
                                        )
                                        .changed();
                                });

                            ui.horizontal(|ui| {
                                ui.label(lang.tr("Старт:", "Start:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_start).range(0..=99999))
                                    .changed();
                                ui.label(lang.tr("Шаг:", "Step:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_step).range(1..=1000))
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(lang.tr("Цифр (001):", "Zero pad:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_pad).range(1..=10))
                                    .changed();
                                ui.label(lang.tr("Раздел.:", "Sep:"));
                                needs_recalc |= ui
                                    .add(
                                        egui::TextEdit::singleline(&mut self.num_sep)
                                            .desired_width(38.0),
                                    )
                                    .changed();
                            });

                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.reset_num_per_dir,
                                    lang.tr(
                                        "Сброс счётчика в подпапках",
                                        "Reset counter per folder",
                                    ),
                                )
                                .changed();
                        });

                        ui.add_space(8.0);

                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr("5. Расширения файлов", "5. File Extensions"),
                                is_dark,
                            );
                            ui.add_space(4.0);

                            ui.label(lang.tr("Режим расширения:", "Extension mode:"));
                            egui::ComboBox::from_id_salt("ext_mode_cb")
                                .width(265.0)
                                .selected_text(match self.ext_mode {
                                    ExtMode::Keep => lang.tr("Не менять", "Keep original"),
                                    ExtMode::Lower => {
                                        lang.tr("в строчные (.jpg)", "lowercase (.jpg)")
                                    }
                                    ExtMode::Upper => {
                                        lang.tr("В ПРОПИСНЫЕ (.JPG)", "UPPERCASE (.JPG)")
                                    }
                                    ExtMode::Remove => {
                                        lang.tr("Удалить расширение", "Remove extension")
                                    }
                                })
                                .show_ui(ui, |ui| {
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.ext_mode,
                                            ExtMode::Keep,
                                            lang.tr("Не менять", "Keep original"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.ext_mode,
                                            ExtMode::Lower,
                                            lang.tr("в строчные (.jpg)", "lowercase (.jpg)"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.ext_mode,
                                            ExtMode::Upper,
                                            lang.tr("В ПРОПИСНЫЕ (.JPG)", "UPPERCASE (.JPG)"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.ext_mode,
                                            ExtMode::Remove,
                                            lang.tr("Удалить расширение", "Remove extension"),
                                        )
                                        .changed();
                                });

                            ui.label(lang.tr(
                                "Заменить расширение на (напр. jpg):",
                                "Replace extension with (e.g. jpg):",
                            ));
                            needs_recalc |= ui.text_edit_singleline(&mut self.new_ext).changed();
                        });

                        ui.add_space(8.0);

                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr("6. Фильтры и исключения", "6. Advanced Filters"),
                                is_dark,
                            );
                            ui.add_space(4.0);

                            ui.label(lang.tr(
                                "Только расширения (jpg, png):",
                                "Include extensions (jpg, png):",
                            ));
                            needs_recalc |= ui.text_edit_singleline(&mut self.ext_filter).changed();

                            ui.label(lang.tr(
                                "Исключить расширения (tmp, bak):",
                                "Exclude extensions (tmp, bak):",
                            ));
                            needs_recalc |=
                                ui.text_edit_singleline(&mut self.ext_exclude_filter).changed();

                            ui.label(lang.tr(
                                "Исключить слова в имени (через запятую):",
                                "Exclude words in name (comma-sep):",
                            ));
                            needs_recalc |= ui
                                .text_edit_singleline(&mut self.exclude_words_filter)
                                .changed();

                            ui.label(lang.tr("Фильтр по размеру:", "File size filter:"));
                            egui::ComboBox::from_id_salt("size_filter_cb")
                                .width(265.0)
                                .selected_text(match self.size_filter_mode {
                                    SizeFilterMode::Any => lang.tr("Любой размер", "Any size"),
                                    SizeFilterMode::GreaterThanKB => {
                                        lang.tr("Больше чем (КБ)", "Greater than (KB)")
                                    }
                                    SizeFilterMode::LessThanKB => {
                                        lang.tr("Меньше чем (КБ)", "Less than (KB)")
                                    }
                                    SizeFilterMode::BetweenKB => {
                                        lang.tr("Диапазон (от / до КБ)", "Between (min / max KB)")
                                    }
                                })
                                .show_ui(ui, |ui| {
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.size_filter_mode,
                                            SizeFilterMode::Any,
                                            lang.tr("Любой размер", "Any size"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.size_filter_mode,
                                            SizeFilterMode::GreaterThanKB,
                                            lang.tr("Больше чем (КБ)", "Greater than (KB)"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.size_filter_mode,
                                            SizeFilterMode::LessThanKB,
                                            lang.tr("Меньше чем (КБ)", "Less than (KB)"),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.size_filter_mode,
                                            SizeFilterMode::BetweenKB,
                                            lang.tr("Диапазон (от / до КБ)", "Between (min / max KB)"),
                                        )
                                        .changed();
                                });

                            if self.size_filter_mode != SizeFilterMode::Any {
                                ui.horizontal(|ui| {
                                    if self.size_filter_mode == SizeFilterMode::GreaterThanKB
                                        || self.size_filter_mode == SizeFilterMode::BetweenKB
                                    {
                                        ui.label(lang.tr("От (КБ):", "Min (KB):"));
                                        needs_recalc |= ui
                                            .add(
                                                egui::DragValue::new(&mut self.size_min_kb)
                                                    .range(0..=10_000_000),
                                            )
                                            .changed();
                                    }
                                    if self.size_filter_mode == SizeFilterMode::LessThanKB
                                        || self.size_filter_mode == SizeFilterMode::BetweenKB
                                    {
                                        ui.label(lang.tr("До (КБ):", "Max (KB):"));
                                        needs_recalc |= ui
                                            .add(
                                                egui::DragValue::new(&mut self.size_max_kb)
                                                    .range(1..=10_000_000),
                                            )
                                            .changed();
                                    }
                                });
                            }
                        });

                        ui.add_space(12.0);

                        let btn_text = egui::RichText::new(
                            lang.tr("✔ Применить изменения (Ctrl+Enter)", "✔ Apply Changes (Ctrl+Enter)"),
                        )
                        .color(egui::Color32::WHITE)
                        .strong()
                        .size(15.0);

                        let apply_btn = egui::Button::new(btn_text)
                            .fill(egui::Color32::from_rgb(18, 118, 215))
                            .rounding(6.0)
                            .min_size(egui::vec2(ui.available_width(), 40.0));

                        if ui.add(apply_btn).clicked() {
                            trigger_apply = true;
                        }
                    });
            });

        // --- ЦЕНТРАЛЬНАЯ ТАБЛИЦА ПРЕДПРОСМОТРА ---
        egui::CentralPanel::default().show(ctx, |ui| {
            if settings_dirty {
                self.save_settings();
            }
            if let Some(target) = single_trash_target.take() {
                self.delete_paths_to_trash(vec![target]);
            }
            if let Some(folder) = navigate_to.take() {
                self.open_new_directory(folder);
            } else if needs_reload {
                if let Some(dir) = self.current_dir.clone() {
                    self.load_directory(dir);
                }
            } else if needs_recalc {
                self.recalculate_preview();
            }

            if trigger_reset {
                self.reset_rules();
            }
            if trigger_apply {
                self.apply_rename();
            }
            if trigger_undo {
                self.undo_rename();
            }

            ui.horizontal(|ui| {
                ui.heading(lang.tr("Файлы и папки для переименования", "Files to Rename"));
                ui.weak(lang.tr(
                    "• Клик — превью • Двойной клик — открыть • Шапка столбца — сортировка",
                    "• Click to preview • Double-click to open • Column header to sort",
                ));
            });
            ui.add_space(4.0);

            let mut toggle_all: Option<bool> = None;
            let mut toggled_item: Option<(PathBuf, bool)> = None;
            let all_included =
                !self.preview_rows.is_empty() && self.preview_rows.iter().all(|r| r.included);

            let arrow = if self.sort_asc { "▲" } else { "▼" };
            let col_type_title = if self.sort_field == SortField::Extension {
                format!("{} {}", lang.tr("Тип", "Type"), arrow)
            } else {
                lang.tr("Тип", "Type").to_string()
            };
            let col_orig_title = if self.sort_field == SortField::NaturalName
                || self.sort_field == SortField::AlphaName
            {
                format!("{} {}", lang.tr("Текущее имя", "Original Name"), arrow)
            } else {
                lang.tr("Текущее имя", "Original Name").to_string()
            };
            let col_new_title = lang.tr("Новое имя (предпросмотр)", "New Name (Preview)");
            let col_status_title = lang.tr("Статус", "Status");
            let col_size_title = if self.sort_field == SortField::Size {
                format!("{} {}", lang.tr("Размер", "Size"), arrow)
            } else {
                lang.tr("Размер", "Size").to_string()
            };
            let col_date_title = if self.sort_field == SortField::ModifiedDate {
                format!("{} {}", lang.tr("Изменён", "Modified"), arrow)
            } else {
                lang.tr("Изменён", "Modified").to_string()
            };

            let status_ready = lang.tr("Готов", "Ready");
            let status_changed = lang.tr("Изменится", "Preview");
            let status_skipped = lang.tr("Пропущен", "Skipped");
            let label_folder = lang.tr("📁 Папка", "📁 Folder");
            let label_file = lang.tr("📄 Файл", "📄 File");

            let current_selected = self.selected_path.clone();
            let use_diff = self.inline_diff;

            card_frame(is_dark).show(ui, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(34.0))
                    .column(Column::initial(92.0).resizable(true))
                    .column(Column::initial(235.0).resizable(true))
                    .column(Column::initial(235.0).resizable(true))
                    .column(Column::initial(90.0).resizable(true))
                    .column(Column::initial(85.0).resizable(true))
                    .column(Column::remainder())
                    .header(28.0, |mut header| {
                        header.col(|ui| {
                            let mut master = all_included;
                            if ui.checkbox(&mut master, "").changed() {
                                toggle_all = Some(master);
                            }
                        });
                        header.col(|ui| {
                            if ui.button(egui::RichText::new(col_type_title).strong()).clicked() {
                                clicked_sort_col = Some(SortField::Extension);
                            }
                        });
                        header.col(|ui| {
                            if ui.button(egui::RichText::new(col_orig_title).strong()).clicked() {
                                clicked_sort_col = Some(SortField::NaturalName);
                            }
                        });
                        header.col(|ui| {
                            ui.strong(col_new_title);
                        });
                        header.col(|ui| {
                            ui.strong(col_status_title);
                        });
                        header.col(|ui| {
                            if ui.button(egui::RichText::new(col_size_title).strong()).clicked() {
                                clicked_sort_col = Some(SortField::Size);
                            }
                        });
                        header.col(|ui| {
                            if ui.button(egui::RichText::new(col_date_title).strong()).clicked() {
                                clicked_sort_col = Some(SortField::ModifiedDate);
                            }
                        });
                    })
                    .body(|body| {
                        body.rows(28.0, self.preview_rows.len(), |mut row| {
                            let item = &self.preview_rows[row.index()];
                            let muted = if is_dark {
                                egui::Color32::from_gray(115)
                            } else {
                                egui::Color32::from_gray(135)
                            };
                            let is_selected =
                                current_selected.as_ref() == Some(&item.old_path);

                            row.col(|ui| {
                                let mut inc = item.included;
                                if ui.checkbox(&mut inc, "").changed() {
                                    toggled_item = Some((item.old_path.clone(), inc));
                                }
                            });

                            row.col(|ui| {
                                let kind_str = if item.is_dir {
                                    label_folder
                                } else {
                                    label_file
                                };
                                if !item.included {
                                    ui.colored_label(muted, kind_str);
                                } else if item.is_dir {
                                    let folder_col = if is_dark {
                                        egui::Color32::from_rgb(245, 180, 65)
                                    } else {
                                        egui::Color32::from_rgb(195, 115, 10)
                                    };
                                    ui.colored_label(folder_col, kind_str);
                                } else {
                                    ui.label(kind_str);
                                }
                            });

                            row.col(|ui| {
                                let resp = if item.included && item.changed && use_diff && !self.recursive {
                                    let job = build_diff_layout_job(
                                        &item.old_name,
                                        &item.new_name,
                                        false,
                                        is_dark,
                                    );
                                    ui.selectable_label(is_selected, job)
                                } else {
                                    let display_orig =
                                        if self.recursive && item.rel_parent != "." {
                                            format!("{}\\{}", item.rel_parent, item.old_name)
                                        } else {
                                            item.old_name.clone()
                                        };
                                    let text = if item.included {
                                        egui::RichText::new(display_orig)
                                    } else {
                                        egui::RichText::new(display_orig).color(muted)
                                    };
                                    ui.selectable_label(is_selected, text)
                                };

                                if resp.clicked() {
                                    inspect_target = Some(item.old_path.clone());
                                }
                                if resp.double_clicked() {
                                    open_sys_target = Some(item.old_path.clone());
                                }
                            });

                            row.col(|ui| {
                                if !item.included {
                                    ui.colored_label(muted, &item.old_name);
                                } else if item.changed {
                                    if use_diff {
                                        let job = build_diff_layout_job(
                                            &item.old_name,
                                            &item.new_name,
                                            true,
                                            is_dark,
                                        );
                                        ui.label(job);
                                    } else {
                                        let (bg_pill, fg_pill) = if is_dark {
                                            (
                                                egui::Color32::from_rgb(20, 82, 58),
                                                egui::Color32::from_rgb(135, 250, 190),
                                            )
                                        } else {
                                            (
                                                egui::Color32::from_rgb(210, 245, 225),
                                                egui::Color32::from_rgb(10, 95, 48),
                                            )
                                        };
                                        egui::Frame::none()
                                            .fill(bg_pill)
                                            .rounding(5.0)
                                            .inner_margin(egui::Margin::symmetric(7.0, 2.5))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(&item.new_name)
                                                        .color(fg_pill)
                                                        .strong(),
                                                );
                                            });
                                    }
                                } else {
                                    ui.label(&item.new_name);
                                }
                            });

                            row.col(|ui| {
                                if !item.included {
                                    ui.colored_label(muted, status_skipped);
                                } else if item.changed {
                                    let ch_col = if is_dark {
                                        egui::Color32::from_rgb(75, 220, 145)
                                    } else {
                                        egui::Color32::from_rgb(18, 140, 75)
                                    };
                                    ui.label(
                                        egui::RichText::new(status_changed).color(ch_col).strong(),
                                    );
                                } else {
                                    ui.colored_label(muted, status_ready);
                                }
                            });

                            row.col(|ui| {
                                let size_str = Self::format_size(item.size, item.is_dir);
                                ui.label(size_str);
                            });

                            row.col(|ui| {
                                let date_str = format_timestamp(item.modified_ts);
                                ui.label(date_str);
                            });
                        });
                    });
            });

            if let Some(target) = open_sys_target.take() {
                open_in_system(&target);
            }
            if let Some(target_path) = inspect_target.take() {
                self.select_for_preview(target_path, ctx);
            } else if let Some(col_field) = clicked_sort_col.take() {
                self.toggle_column_sort(col_field);
            } else if let Some(select_all) = toggle_all.take() {
                if select_all {
                    for row in &self.preview_rows {
                        self.excluded_paths.remove(&row.old_path);
                    }
                } else {
                    for row in &self.preview_rows {
                        self.excluded_paths.insert(row.old_path.clone());
                    }
                }
                self.recalculate_preview();
            } else if let Some((path, include)) = toggled_item.take() {
                if include {
                    self.excluded_paths.remove(&path);
                } else {
                    self.excluded_paths.insert(path);
                }
                self.recalculate_preview();
            }
        });
    }
}
