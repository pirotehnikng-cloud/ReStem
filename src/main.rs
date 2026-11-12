#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use regex::RegexBuilder;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

fn main() -> eframe::Result<()> {
    let icon = build_app_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ReStem — Пакетное переименование файлов и папок")
            .with_inner_size([1340.0, 820.0])
            .with_min_inner_size([1020.0, 620.0])
            .with_drag_and_drop(true)
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "ReStem",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(ReStemApp::default()))
        }),
    )
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

/// Форматирование UNIX-времени в строку ГГГГ-ММ-ДД ЧЧ:ММ
fn format_timestamp(secs: u64) -> String {
    if secs == 0 {
        return "—".to_string();
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

    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, m, d, hours, mins)
}

/// Транслитерация кириллицы (RU -> LAT) с сохранением регистра
fn transliterate_cyrillic(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for ch in input.chars() {
        let replacement = match ch {
            'а' => "a",  'А' => "A",
            'б' => "b",  'Б' => "B",
            'в' => "v",  'В' => "V",
            'г' => "g",  'Г' => "G",
            'д' => "d",  'Д' => "D",
            'е' => "e",  'Е' => "E",
            'ё' => "yo", 'Ё' => "Yo",
            'ж' => "zh", 'Ж' => "Zh",
            'з' => "z",  'З' => "Z",
            'и' => "i",  'И' => "I",
            'й' => "y",  'Й' => "Y",
            'к' => "k",  'К' => "K",
            'л' => "l",  'Л' => "L",
            'м' => "m",  'М' => "M",
            'н' => "n",  'Н' => "N",
            'о' => "o",  'О' => "O",
            'п' => "p",  'П' => "P",
            'р' => "r",  'Р' => "R",
            'с' => "s",  'С' => "S",
            'т' => "t",  'Т' => "T",
            'у' => "u",  'У' => "U",
            'ф' => "f",  'Ф' => "F",
            'х' => "kh", 'Х' => "Kh",
            'ц' => "ts", 'Ц' => "Ts",
            'ч' => "ch", 'Ч' => "Ch",
            'ш' => "sh", 'Ш' => "Sh",
            'щ' => "shch", 'Щ' => "Shch",
            'ъ' | 'Ъ' | 'ь' | 'Ь' => "",
            'ы' => "y",  'Ы' => "Y",
            'э' => "e",  'Э' => "E",
            'ю' => "yu", 'Ю' => "Yu",
            'я' => "ya", 'Я' => "Ya",
            _ => {
                out.push(ch);
                continue;
            }
        };
        out.push_str(replacement);
    }
    out
}

fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "ico" | "gif"
    )
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
    rel_parent: String,
    name: String,
    ext_lower: String,
    is_dir: bool,
    is_hidden: bool,
    depth: usize,
    size: u64,
    modified_ts: u64,
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

    current_dir: Option<PathBuf>,
    subfolders: Vec<PathBuf>,
    all_entries: Vec<FsEntry>,
    preview_rows: Vec<PreviewRow>,
    excluded_paths: HashSet<PathBuf>,
    undo_history: Vec<(PathBuf, PathBuf)>,

    // Предпросмотр и удаление
    selected_path: Option<PathBuf>,
    preview_state: PreviewState,
    confirm_trash_modal: bool,

    // Настройки сортировки
    sort_field: SortField,
    sort_asc: bool,
    folders_first: bool,

    // Быстрые фильтры верхней панели
    search_query: String,
    search_regex: bool,
    search_case_sensitive: bool,
    status_filter: StatusFilter,
    include_files: bool,
    include_dirs: bool,
    recursive: bool,
    hide_system_hidden: bool,

    // Расширенные фильтры (Блок 6)
    ext_filter: String,
    ext_exclude_filter: String,
    exclude_words_filter: String,
    size_filter_mode: SizeFilterMode,
    size_min_kb: u64,
    size_max_kb: u64,

    // 1. Поиск и замена
    find_str: String,
    replace_str: String,
    use_regex: bool,
    case_sensitive: bool,

    // 2. Регистр и транслитерация
    case_mode: CaseMode,
    transliterate: bool,

    // 3. Префикс, суффикс и обрезка
    prefix: String,
    suffix: String,
    trim_start: usize,
    trim_end: usize,

    // 4. Нумерация
    num_mode: NumMode,
    num_start: usize,
    num_step: usize,
    num_pad: usize,
    num_sep: String,
    reset_num_per_dir: bool,

    // 5. Расширение файлов
    ext_mode: ExtMode,
    new_ext: String,

    status_msg: String,
}

impl Default for ReStemApp {
    fn default() -> Self {
        Self {
            lang: Lang::Ru,
            dark_mode: false,
            current_dir: None,
            subfolders: Vec::new(),
            all_entries: Vec::new(),
            preview_rows: Vec::new(),
            excluded_paths: HashSet::new(),
            undo_history: Vec::new(),

            selected_path: None,
            preview_state: PreviewState::Empty,
            confirm_trash_modal: false,

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
            transliterate: false,
            prefix: String::new(),
            suffix: String::new(),
            trim_start: 0,
            trim_end: 0,
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

        // 1. Попытка загрузить превью картинки
        if is_image_ext(&ext) {
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
                    let texture =
                        ctx.load_texture("item_preview_img", color_img, egui::TextureOptions::LINEAR);
                    self.preview_state = PreviewState::Image {
                        name,
                        texture,
                        orig_w,
                        orig_h,
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

        // 2. Попытка прочитать текстовый фрагмент (до 4096 байт)
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

            out.push(FsEntry {
                path: path.clone(),
                parent_dir,
                rel_parent: rel_display,
                name,
                ext_lower,
                is_dir,
                is_hidden,
                depth,
                size,
                modified_ts,
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

    fn compute_new_name(&self, entry: &FsEntry, idx: usize) -> String {
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
                stem = re.replace_all(&stem, self.replace_str.as_str()).to_string();
            }
        }

        stem = format!("{}{}{}", self.prefix, stem, self.suffix);

        if self.transliterate {
            stem = transliterate_cyrillic(&stem);
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

            if self.transliterate {
                ext_val = transliterate_cyrillic(&ext_val);
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
                let idx = if self.reset_num_per_dir {
                    let c = dir_counters.entry(entry.parent_dir.clone()).or_insert(0);
                    let current = *c;
                    *c += 1;
                    current
                } else {
                    let current = global_counter;
                    global_counter += 1;
                    current
                };
                self.compute_new_name(entry, idx)
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
                "Показано: {} (из {})  |  Выбрано: {}  |  К переименованию: {}",
                self.preview_rows.len(),
                self.all_entries.len(),
                included_count,
                changed_count
            ),
            Lang::En => format!(
                "Shown: {} (of {})  |  Selected: {}  |  Ready to rename: {}",
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
        self.transliterate = false;
        self.prefix.clear();
        self.suffix.clear();
        self.trim_start = 0;
        self.trim_end = 0;
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

        let mut history = Vec::new();
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
            history.push((new_path, row.old_path));
        }

        let count = history.len();
        self.undo_history = history;
        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir);
        }
        self.status_msg = match self.lang {
            Lang::Ru => format!("Успешно переименовано объектов: {}", count),
            Lang::En => format!("Successfully renamed items: {}", count),
        };
    }

    fn undo_rename(&mut self) {
        let mut restored = 0;
        for (curr, orig) in self.undo_history.drain(..).rev() {
            if fs::rename(&curr, &orig).is_ok() {
                restored += 1;
            }
        }
        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir);
        }
        self.status_msg = match self.lang {
            Lang::Ru => format!("Откат выполнен: возвращено {} имён", restored),
            Lang::En => format!("Undo complete: restored {} names", restored),
        };
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

        let lang = self.lang;
        let mut needs_recalc = false;
        let mut needs_reload = false;
        let mut navigate_to: Option<PathBuf> = None;
        let mut trigger_apply = false;
        let mut trigger_undo = false;
        let mut trigger_reset = false;
        let mut clicked_sort_col: Option<SortField> = None;
        let mut inspect_target: Option<PathBuf> = None;
        let mut single_trash_target: Option<PathBuf> = None;

        let checked_count = self.preview_rows.iter().filter(|r| r.included).count();

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
                    ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 5.0, egui::Color32::from_rgb(20, 30, 48));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "R",
                    egui::FontId::proportional(16.0),
                    egui::Color32::from_rgb(35, 205, 175),
                );
                ui.strong(egui::RichText::new("ReStem").size(16.0));
                ui.separator();

                if ui
                    .button(lang.tr("📁 Открыть папку...", "📁 Add Folder..."))
                    .clicked()
                {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        navigate_to = Some(folder);
                    }
                }

                if ui.button(lang.tr("🔄 Обновить", "🔄 Refresh")).clicked() {
                    needs_reload = true;
                }

                if ui
                    .button(lang.tr("☑ Инвертировать", "☑ Invert"))
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

                if ui
                    .add_enabled(
                        !self.undo_history.is_empty(),
                        egui::Button::new(lang.tr("⮌ Откат (Undo)", "⮌ Undo")),
                    )
                    .clicked()
                {
                    trigger_undo = true;
                }

                let trash_label = match lang {
                    Lang::Ru => format!("🗑 В корзину ({})", checked_count),
                    Lang::En => format!("🗑 Trash ({})", checked_count),
                };
                if ui
                    .add_enabled(checked_count > 0, egui::Button::new(trash_label))
                    .on_hover_text(lang.tr(
                        "Переместить отмеченные галочками файлы и папки в Корзину Windows",
                        "Move checked files and folders to the Windows Recycle Bin",
                    ))
                    .clicked()
                {
                    self.confirm_trash_modal = true;
                }

                ui.separator();

                ui.label("🔍");
                let search_hint = lang.tr("Поиск по имени...", "Filter by name...");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(search_hint)
                            .desired_width(145.0),
                    )
                    .changed()
                {
                    needs_recalc = true;
                }
                if ui
                    .selectable_label(self.search_case_sensitive, "Aa")
                    .on_hover_text(lang.tr("Учитывать регистр в поиске", "Case-sensitive search"))
                    .clicked()
                {
                    self.search_case_sensitive = !self.search_case_sensitive;
                    needs_recalc = true;
                }
                if ui
                    .selectable_label(self.search_regex, ".*")
                    .on_hover_text(lang.tr(
                        "Использовать Regex в поиске",
                        "Use Regex in search filter",
                    ))
                    .clicked()
                {
                    self.search_regex = !self.search_regex;
                    needs_recalc = true;
                }
                if !self.search_query.is_empty() && ui.small_button("✖").clicked() {
                    self.search_query.clear();
                    needs_recalc = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .selectable_label(self.lang == Lang::En, "EN")
                        .on_hover_text("Switch to English")
                        .clicked()
                    {
                        self.lang = Lang::En;
                        needs_recalc = true;
                    }
                    if ui
                        .selectable_label(self.lang == Lang::Ru, "RU")
                        .on_hover_text("Переключить на русский язык")
                        .clicked()
                    {
                        self.lang = Lang::Ru;
                        needs_recalc = true;
                    }

                    ui.separator();

                    let theme_label = if self.dark_mode {
                        lang.tr("🌙 Тёмная", "🌙 Dark")
                    } else {
                        lang.tr("☀ Светлая", "☀ Light")
                    };
                    if ui
                        .button(theme_label)
                        .on_hover_text(lang.tr(
                            "Сменить тему оформления",
                            "Toggle Light/Dark theme",
                        ))
                        .clicked()
                    {
                        self.dark_mode = !self.dark_mode;
                        if self.dark_mode {
                            ctx.set_visuals(egui::Visuals::dark());
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        }
                    }
                });
            });

            ui.separator();

            // Ряд 2: Панель фильтрации и сортировки
            ui.horizontal(|ui| {
                ui.strong(lang.tr("Объекты:", "Items:"));
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
                        lang.tr("Скрыть системные/.git", "Hide system/.git"),
                    )
                    .changed()
                {
                    needs_reload = true;
                }

                ui.separator();

                ui.label(lang.tr("Показать:", "Show:"));
                egui::ComboBox::from_id_salt("status_filter_cb")
                    .width(150.0)
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

                ui.strong(lang.tr("Сортировка:", "Sort by:"));
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
                    lang.tr("⬆ По возр.", "⬆ Asc")
                } else {
                    lang.tr("⬇ По убыв.", "⬇ Desc")
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
            ui.add_space(2.0);
        });

        // --- НИЖНЯЯ СТРОКА СОСТОЯНИЯ ---
        egui::TopBottomPanel::bottom("status_footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let path_str = self
                        .current_dir
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| {
                            lang.tr("Папка не выбрана", "No folder selected").to_string()
                        });
                    ui.weak(path_str);
                });
            });
        });

        // --- ЛЕВАЯ КОЛОНКА: ПАПКИ, ШАБЛОНЫ И ПРЕДПРОСМОТР ФАЙЛА ---
        egui::SidePanel::left("left_explorer_panel")
            .resizable(true)
            .default_width(255.0)
            .width_range(210.0..=380.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("left_panel_main_scroll")
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        ui.heading(lang.tr("Папки", "Folders"));
                        ui.separator();

                        if let Some(curr) = &self.current_dir {
                            if let Some(parent) = curr.parent() {
                                if ui
                                    .button(lang.tr("⬆ На уровень вверх (..)", "⬆ Up one level (..)"))
                                    .clicked()
                                {
                                    navigate_to = Some(parent.to_path_buf());
                                }
                            }
                            let curr_name = curr
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| curr.display().to_string());
                            ui.strong(format!("📂 {}", curr_name));
                        } else {
                            ui.weak(lang.tr(
                                "Нажмите «Открыть папку» или перетащите каталог сюда",
                                "Click 'Add Folder' or drag a folder here",
                            ));
                        }

                        egui::ScrollArea::vertical()
                            .id_salt("folders_sub_scroll")
                            .max_height(155.0)
                            .show(ui, |ui| {
                                for sub in &self.subfolders {
                                    let name = sub
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    if ui.selectable_label(false, format!("  📁 {}", name)).clicked()
                                    {
                                        navigate_to = Some(sub.clone());
                                    }
                                }
                            });

                        ui.add_space(10.0);
                        ui.heading(lang.tr("Предпросмотр объекта", "Item Preview"));
                        ui.separator();

                        ui.group(|ui| {
                            ui.set_min_width(ui.available_width());
                            match &self.preview_state {
                                PreviewState::Empty => {
                                    ui.weak(lang.tr(
                                        "Кликните по любому файлу или папке в таблице для предпросмотра картинки или текста.",
                                        "Click any file or folder in the table to preview image or text.",
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
                                    tex_size,
                                } => {
                                    ui.strong(format!("🖼 {}", name));
                                    ui.weak(format!("{} × {} px", orig_w, orig_h));
                                    let max_w = ui.available_width().max(80.0);
                                    let max_h = 185.0;
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
                                            "Показаны первые 4 КБ текста:",
                                            "Showing first 4 KB of text:",
                                        ));
                                    }
                                    egui::ScrollArea::vertical()
                                        .id_salt("text_preview_scroll")
                                        .max_height(165.0)
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(content)
                                                        .monospace()
                                                        .size(11.5),
                                                )
                                                .wrap(),
                                            );
                                        });
                                }
                                PreviewState::BinaryOrOther { name, info } => {
                                    ui.strong(format!("📄 {}", name));
                                    ui.weak(info);
                                }
                            }

                            if let Some(sel_path) = &self.selected_path {
                                ui.add_space(6.0);
                                if ui
                                    .button(lang.tr(
                                        "🗑 Удалить этот объект в Корзину",
                                        "🗑 Move this item to Recycle Bin",
                                    ))
                                    .clicked()
                                {
                                    single_trash_target = Some(sel_path.clone());
                                }
                            }
                        });

                        ui.add_space(10.0);
                        ui.heading(lang.tr("Шаблоны правил", "Saved Presets"));
                        ui.separator();

                        if ui
                            .button(lang.tr(
                                "🔹 Транслитерация + без пробелов",
                                "🔹 Transliterate RU → LAT + _",
                            ))
                            .clicked()
                        {
                            self.transliterate = true;
                            self.find_str = " ".to_string();
                            self.replace_str = "_".to_string();
                            self.use_regex = false;
                            needs_recalc = true;
                        }
                        if ui
                            .button(lang.tr(
                                "🔹 Пробелы → подчёркивания (_)",
                                "🔹 Spaces to underscores (_)",
                            ))
                            .clicked()
                        {
                            self.find_str = " ".to_string();
                            self.replace_str = "_".to_string();
                            self.use_regex = false;
                            needs_recalc = true;
                        }
                        if ui
                            .button(lang.tr(
                                "🔹 Нумерация по дате съёмки/изм.",
                                "🔹 Sequence sorted by date",
                            ))
                            .clicked()
                        {
                            self.sort_field = SortField::ModifiedDate;
                            self.sort_asc = true;
                            self.num_mode = NumMode::Prefix;
                            self.num_start = 1;
                            self.num_pad = 3;
                            self.num_sep = "_".to_string();
                            needs_recalc = true;
                        }
                        if ui
                            .button(lang.tr(
                                "🔹 Строчные имена и расширения",
                                "🔹 Lowercase name & extension",
                            ))
                            .clicked()
                        {
                            self.case_mode = CaseMode::Lower;
                            self.ext_mode = ExtMode::Lower;
                            needs_recalc = true;
                        }
                        if ui
                            .button(lang.tr(
                                "🔹 Убрать копии « (1)», « (2)»",
                                "🔹 Remove ' (1)', ' (2)' copies",
                            ))
                            .clicked()
                        {
                            self.find_str = r"\s*\(\d+\)".to_string();
                            self.replace_str = String::new();
                            self.use_regex = true;
                            needs_recalc = true;
                        }

                        ui.add_space(6.0);
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

        // --- ПРАВАЯ КОЛОНКА: ИНСПЕКТОР ПРАВИЛ И ФИЛЬТРОВ ---
        egui::SidePanel::right("right_rules_panel")
            .resizable(true)
            .default_width(310.0)
            .width_range(270.0..=430.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.heading(lang.tr("Правила переименования", "Rename Rules"));
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("rules_scroll")
                    .show(ui, |ui| {
                        ui.group(|ui| {
                            ui.strong(lang.tr("1. Поиск и замена", "1. Find & Replace"));
                            ui.add_space(2.0);
                            ui.label(lang.tr("Найти:", "Find:"));
                            needs_recalc |= ui.text_edit_singleline(&mut self.find_str).changed();
                            ui.label(lang.tr("Заменить на:", "Replace with:"));
                            needs_recalc |= ui.text_edit_singleline(&mut self.replace_str).changed();
                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.use_regex,
                                    lang.tr("Регулярные выражения (Regex)", "Use Regular Expressions"),
                                )
                                .changed();
                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.case_sensitive,
                                    lang.tr("Учитывать регистр", "Case Sensitive"),
                                )
                                .changed();
                        });

                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.strong(lang.tr(
                                "2. Регистр и транслитерация",
                                "2. Case & Transliteration",
                            ));
                            ui.add_space(2.0);
                            egui::ComboBox::from_id_salt("case_cb")
                                .width(250.0)
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

                            ui.add_space(2.0);
                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.transliterate,
                                    lang.tr(
                                        "Транслитерация кириллицы (RU → LAT)",
                                        "Transliterate Cyrillic (RU → LAT)",
                                    ),
                                )
                                .changed();
                        });

                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.strong(lang.tr(
                                "3. Префикс, суффикс и обрезка",
                                "3. Add Prefix/Suffix & Trim",
                            ));
                            ui.add_space(2.0);
                            egui::Grid::new("prefix_grid").num_columns(2).show(ui, |ui| {
                                ui.label(lang.tr("Префикс:", "Prefix:"));
                                needs_recalc |= ui.text_edit_singleline(&mut self.prefix).changed();
                                ui.end_row();

                                ui.label(lang.tr("Суффикс:", "Suffix:"));
                                needs_recalc |= ui.text_edit_singleline(&mut self.suffix).changed();
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
                        });

                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.strong(lang.tr(
                                "4. Нумерация (по порядку сортировки)",
                                "4. Numbering (follows sort order)",
                            ));
                            ui.add_space(2.0);
                            egui::ComboBox::from_id_salt("num_cb")
                                .width(250.0)
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
                                            .desired_width(36.0),
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

                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.strong(lang.tr("5. Расширения файлов", "5. File Extensions"));
                            ui.add_space(2.0);

                            ui.label(lang.tr("Режим расширения:", "Extension mode:"));
                            egui::ComboBox::from_id_salt("ext_mode_cb")
                                .width(250.0)
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

                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.strong(lang.tr(
                                "6. Фильтры и исключения",
                                "6. Advanced Filters",
                            ));
                            ui.add_space(2.0);

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
                                .width(250.0)
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
                            lang.tr("✔ Применить изменения", "✔ Apply Changes"),
                        )
                        .color(egui::Color32::WHITE)
                        .strong()
                        .size(15.0);

                        let apply_btn = egui::Button::new(btn_text)
                            .fill(egui::Color32::from_rgb(18, 118, 215))
                            .min_size(egui::vec2(ui.available_width(), 38.0));

                        if ui.add(apply_btn).clicked() {
                            trigger_apply = true;
                        }
                    });
            });

        // --- ЦЕНТРАЛЬНАЯ ТАБЛИЦА ПРЕДПРОСМОТРА ---
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(target) = single_trash_target {
                self.delete_paths_to_trash(vec![target]);
            }
            if let Some(folder) = navigate_to {
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
                    "(клик по имени — предпросмотр картинки/текста, клик по шапке — сортировка)",
                    "(click name to preview image/text, click header to sort)",
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
                lang.tr("Тип ↕", "Type ↕").to_string()
            };
            let col_orig_title = if self.sort_field == SortField::NaturalName
                || self.sort_field == SortField::AlphaName
            {
                format!("{} {}", lang.tr("Текущее имя", "Original Name"), arrow)
            } else {
                lang.tr("Текущее имя ↕", "Original Name ↕").to_string()
            };
            let col_new_title = lang.tr("Новое имя", "New Name");
            let col_status_title = lang.tr("Статус", "Status");
            let col_size_title = if self.sort_field == SortField::Size {
                format!("{} {}", lang.tr("Размер", "Size"), arrow)
            } else {
                lang.tr("Размер ↕", "Size ↕").to_string()
            };
            let col_date_title = if self.sort_field == SortField::ModifiedDate {
                format!("{} {}", lang.tr("Изменён", "Modified"), arrow)
            } else {
                lang.tr("Изменён ↕", "Modified ↕").to_string()
            };

            let status_ready = lang.tr("Готов", "Ready");
            let status_changed = lang.tr("Изменится", "Preview");
            let status_skipped = lang.tr("Пропущен", "Skipped");
            let label_folder = lang.tr("📁 Папка", "📁 Folder");
            let label_file = lang.tr("📄 Файл", "📄 File");

            let is_dark = self.dark_mode;
            let current_selected = self.selected_path.clone();

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(32.0))
                .column(Column::initial(85.0).resizable(true))
                .column(Column::initial(220.0).resizable(true))
                .column(Column::initial(220.0).resizable(true))
                .column(Column::initial(80.0).resizable(true))
                .column(Column::initial(80.0).resizable(true))
                .column(Column::remainder())
                .header(24.0, |mut header| {
                    header.col(|ui| {
                        let mut master = all_included;
                        if ui.checkbox(&mut master, "").changed() {
                            toggle_all = Some(master);
                        }
                    });
                    header.col(|ui| {
                        if ui.button(col_type_title).clicked() {
                            clicked_sort_col = Some(SortField::Extension);
                        }
                    });
                    header.col(|ui| {
                        if ui.button(col_orig_title).clicked() {
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
                        if ui.button(col_size_title).clicked() {
                            clicked_sort_col = Some(SortField::Size);
                        }
                    });
                    header.col(|ui| {
                        if ui.button(col_date_title).clicked() {
                            clicked_sort_col = Some(SortField::ModifiedDate);
                        }
                    });
                })
                .body(|body| {
                    body.rows(24.0, self.preview_rows.len(), |mut row| {
                        let item = &self.preview_rows[row.index()];
                        let muted = egui::Color32::from_gray(130);
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
                                ui.colored_label(egui::Color32::from_rgb(215, 140, 25), kind_str);
                            } else {
                                ui.label(kind_str);
                            }
                        });

                        row.col(|ui| {
                            let display_orig = if self.recursive && item.rel_parent != "." {
                                format!("{}\\{}", item.rel_parent, item.old_name)
                            } else {
                                item.old_name.clone()
                            };
                            let text = if item.included {
                                egui::RichText::new(display_orig)
                            } else {
                                egui::RichText::new(display_orig).color(muted)
                            };
                            if ui.selectable_label(is_selected, text).clicked() {
                                inspect_target = Some(item.old_path.clone());
                            }
                        });

                        row.col(|ui| {
                            if !item.included {
                                ui.colored_label(muted, &item.old_name);
                            } else if item.changed {
                                let (bg_pill, fg_pill) = if is_dark {
                                    (
                                        egui::Color32::from_rgb(22, 75, 55),
                                        egui::Color32::from_rgb(130, 245, 185),
                                    )
                                } else {
                                    (
                                        egui::Color32::from_rgb(212, 244, 226),
                                        egui::Color32::from_rgb(14, 105, 55),
                                    )
                                };
                                egui::Frame::none()
                                    .fill(bg_pill)
                                    .rounding(4.0)
                                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.colored_label(fg_pill, &item.new_name);
                                    });
                            } else {
                                ui.label(&item.new_name);
                            }
                        });

                        row.col(|ui| {
                            if !item.included {
                                ui.colored_label(muted, status_skipped);
                            } else if item.changed {
                                ui.colored_label(
                                    egui::Color32::from_rgb(28, 160, 95),
                                    status_changed,
                                );
                            } else {
                                ui.weak(status_ready);
                            }
                        });

                        row.col(|ui| {
                            let size_str = Self::format_size(item.size, item.is_dir);
                            ui.weak(size_str);
                        });

                        row.col(|ui| {
                            let date_str = format_timestamp(item.modified_ts);
                            ui.weak(date_str);
                        });
                    });
                });

            if let Some(target_path) = inspect_target {
                self.select_for_preview(target_path, ctx);
            } else if let Some(col_field) = clicked_sort_col {
                self.toggle_column_sort(col_field);
            } else if let Some(select_all) = toggle_all {
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
            } else if let Some((path, include)) = toggled_item {
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
