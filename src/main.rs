#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use regex::RegexBuilder;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let icon = build_app_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ReStem — Пакетное переименование файлов и папок")
            .with_inner_size([1240.0, 760.0])
            .with_min_inner_size([960.0, 560.0])
            .with_drag_and_drop(true)
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "ReStem",
        options,
        Box::new(|cc| {
            // Светлая тема по умолчанию (Вариант 4)
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(ReStemApp::default()))
        }),
    )
}

/// Программная генерация иконки ReStem (32x32 RGBA) в бирюзово-изумрудной гамме
fn build_app_icon() -> egui::IconData {
    let width = 32;
    let height = 32;
    let mut rgba = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            let dx = (x as f32 - 15.5).abs();
            let dy = (y as f32 - 15.5).abs();

            // Скруглённый тёмно-синий квадрат фона
            if dx < 14.5 && dy < 14.5 && (dx < 11.5 || dy < 11.5 || (dx - 11.5).hypot(dy - 11.5) <= 3.0) {
                rgba[idx] = 20;
                rgba[idx + 1] = 30;
                rgba[idx + 2] = 48;
                rgba[idx + 3] = 255;

                // Стилизованная буква R со стрелкой (градиент от бирюзового к изумрудному)
                let in_left_stem = (8..=12).contains(&x) && (7..=24).contains(&y);
                let in_top_bar = (12..=20).contains(&x) && (7..=10).contains(&y);
                let in_mid_bar = (12..=20).contains(&x) && (14..=17).contains(&y);
                let in_right_loop = (19..=23).contains(&x) && (9..=15).contains(&y);
                let in_leg = (y >= 17 && y <= 24) && ((x as i32 - (y as i32 - 3)).abs() <= 2);

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

#[derive(PartialEq, Clone, Copy)]
enum Lang {
    Ru,
    En,
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

#[derive(Clone)]
struct FsEntry {
    path: PathBuf,
    parent_dir: PathBuf,
    rel_parent: String,
    is_dir: bool,
    depth: usize,
    size: u64,
}

#[derive(Clone)]
struct PreviewRow {
    old_path: PathBuf,
    parent_dir: PathBuf,
    rel_parent: String,
    is_dir: bool,
    depth: usize,
    size: u64,
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

    // Фильтры верхней панели
    search_query: String,
    include_files: bool,
    include_dirs: bool,
    recursive: bool,
    ext_filter: String,

    // 1. Поиск и замена
    find_str: String,
    replace_str: String,
    use_regex: bool,
    case_sensitive: bool,

    // 2. Добавление и обрезка
    prefix: String,
    suffix: String,
    trim_start: usize,
    trim_end: usize,

    // 3. Регистр и нумерация
    case_mode: CaseMode,
    num_mode: NumMode,
    num_start: usize,
    num_step: usize,
    num_pad: usize,
    num_sep: String,
    reset_num_per_dir: bool,

    status_msg: String,
}

impl Default for ReStemApp {
    fn default() -> Self {
        Self {
            lang: Lang::Ru,
            dark_mode: false, // Светлая тема по умолчанию (Вариант 4)
            current_dir: None,
            subfolders: Vec::new(),
            all_entries: Vec::new(),
            preview_rows: Vec::new(),
            excluded_paths: HashSet::new(),
            undo_history: Vec::new(),
            search_query: String::new(),
            include_files: true,
            include_dirs: true,
            recursive: false,
            ext_filter: String::new(),
            find_str: String::new(),
            replace_str: String::new(),
            use_regex: false,
            case_sensitive: true,
            prefix: String::new(),
            suffix: String::new(),
            trim_start: 0,
            trim_end: 0,
            case_mode: CaseMode::None,
            num_mode: NumMode::None,
            num_start: 1,
            num_step: 1,
            num_pad: 2,
            num_sep: "_".to_string(),
            reset_num_per_dir: true,
            status_msg: "Выберите папку или перетащите её мышью в окно программы.".to_string(),
        }
    }
}

impl ReStemApp {
    fn tr<'a>(&self, ru: &'a str, en: &'a str) -> &'a str {
        match self.lang {
            Lang::Ru => ru,
            Lang::En => en,
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

    fn open_new_directory(&mut self, root_dir: PathBuf) {
        if self.current_dir.as_ref() != Some(&root_dir) {
            self.excluded_paths.clear();
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
                    subs.push(p);
                }
            }
        }
        subs.sort_by_key(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        });
        self.subfolders = subs;

        self.collect_entries(&root_dir, &root_dir, 0, &mut entries);

        entries.sort_by(|a, b| {
            a.rel_parent
                .to_lowercase()
                .cmp(&b.rel_parent.to_lowercase())
                .then_with(|| b.is_dir.cmp(&a.is_dir))
                .then_with(|| {
                    a.path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                        .cmp(
                            &b.path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase(),
                        )
                })
        });

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

        let mut local_entries: Vec<(PathBuf, u64, bool)> = read_dir
            .flatten()
            .map(|e| {
                let path = e.path();
                let meta = e.metadata().ok();
                let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or_else(|| path.is_dir());
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                (path, size, is_dir)
            })
            .collect();

        local_entries.sort_by_key(|(p, _, _)| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        });

        for (path, size, is_dir) in local_entries {
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

            out.push(FsEntry {
                path: path.clone(),
                parent_dir,
                rel_parent: rel_display,
                is_dir,
                depth,
                size,
            });

            if is_dir && self.recursive {
                self.collect_entries(root, &path, depth + 1, out);
            }
        }
    }

    fn compute_new_name(&self, entry: &FsEntry, idx: usize) -> String {
        let (mut stem, ext) = if entry.is_dir {
            let full_name = entry
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            (full_name, String::new())
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
                .map(|ext_os| format!(".{}", ext_os.to_string_lossy()))
                .unwrap_or_default();
            (s, e)
        };

        // 1. Обрезка символов
        let chars: Vec<char> = stem.chars().collect();
        let start = self.trim_start.min(chars.len());
        let end = chars.len().saturating_sub(self.trim_end).max(start);
        stem = chars[start..end].iter().collect();

        // 2. Поиск и замена
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

        // 3. Префикс и суффикс
        stem = format!("{}{}{}", self.prefix, stem, self.suffix);

        // 4. Регистр
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
                        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        };

        // 5. Нумерация
        if self.num_mode != NumMode::None {
            let num = self.num_start + idx * self.num_step.max(1);
            let num_str = format!("{:0width$}", num, width = self.num_pad.max(1));
            stem = match self.num_mode {
                NumMode::Suffix if !stem.is_empty() => format!("{}{}{}", stem, self.num_sep, num_str),
                NumMode::Prefix if !stem.is_empty() => format!("{}{}{}", num_str, self.num_sep, stem),
                _ => num_str,
            };
        }

        format!("{}{}", stem, ext)
    }

    fn recalculate_preview(&mut self) {
        let allowed_exts: Vec<String> = self
            .ext_filter
            .split(',')
            .map(|s| s.trim().trim_start_matches('.').to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let search_lower = self.search_query.trim().to_lowercase();

        let filtered: Vec<&FsEntry> = self
            .all_entries
            .iter()
            .filter(|e| {
                if e.is_dir && !self.include_dirs {
                    return false;
                }
                if !e.is_dir && !self.include_files {
                    return false;
                }
                if !search_lower.is_empty() {
                    let name_lower = e
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    if !name_lower.contains(&search_lower) {
                        return false;
                    }
                }
                if !allowed_exts.is_empty() {
                    if e.is_dir {
                        return false;
                    }
                    let ext = e
                        .path
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    return allowed_exts.contains(&ext);
                }
                true
            })
            .collect();

        self.preview_rows.clear();
        let mut changed_count = 0;
        let mut included_count = 0;
        let mut last_parent: Option<PathBuf> = None;
        let mut counter_idx = 0;

        for entry in filtered {
            if self.reset_num_per_dir {
                if last_parent.as_ref() != Some(&entry.parent_dir) {
                    counter_idx = 0;
                    last_parent = Some(entry.parent_dir.clone());
                }
            }

            let old_name = entry
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let included = !self.excluded_paths.contains(&entry.path);
            let new_name = if included {
                included_count += 1;
                let computed = self.compute_new_name(entry, counter_idx);
                counter_idx += 1;
                computed
            } else {
                old_name.clone()
            };

            let changed = included && old_name != new_name && !new_name.is_empty();
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
                old_name,
                new_name,
                included,
                changed,
            });
        }

        self.status_msg = match self.lang {
            Lang::Ru => format!(
                "Всего: {}  |  Выбрано: {}  |  К переименованию: {}",
                self.preview_rows.len(),
                included_count,
                changed_count
            ),
            Lang::En => format!(
                "Total: {}  |  Selected: {}  |  Ready to rename: {}",
                self.preview_rows.len(),
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
        self.prefix.clear();
        self.suffix.clear();
        self.trim_start = 0;
        self.trim_end = 0;
        self.case_mode = CaseMode::None;
        self.num_mode = NumMode::None;
        self.num_start = 1;
        self.num_step = 1;
        self.num_pad = 2;
        self.num_sep = "_".to_string();
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
                    Lang::Ru => format!("Ошибка: дубликат имени '{}' в '{}'", final_name, row.rel_parent),
                    Lang::En => format!("Error: duplicate name '{}' in '{}'", final_name, row.rel_parent),
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
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drag_and_drop_overlay"),
            ));
            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(185));
            painter.text(
                screen_rect.center(),
                egui::Align2::CENTER_CENTER,
                self.tr(
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

        let mut needs_recalc = false;
        let mut needs_reload = false;
        let mut navigate_to: Option<PathBuf> = None;
        let mut trigger_apply = false;
        let mut trigger_undo = false;
        let mut trigger_reset = false;

        // --- ВЕРХНЯЯ ПАНЕЛЬ ИНСТРУМЕНТОВ (Вариант 4) ---
        egui::TopBottomPanel::top("top_toolbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // Мини-логотип ReStem в углу
                let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 5.0, egui::Color32::from_rgb(20, 30, 48));
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
                    .button(self.tr("📁 Открыть папку...", "📁 Add Folder..."))
                    .clicked()
                {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        navigate_to = Some(folder);
                    }
                }

                if ui
                    .button(self.tr("🔄 Обновить", "🔄 Refresh"))
                    .clicked()
                {
                    needs_reload = true;
                }

                if ui
                    .button(self.tr("☑ Инвертировать", "☑ Invert"))
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
                        egui::Button::new(self.tr("⮌ Откат (Undo)", "⮌ Undo")),
                    )
                    .clicked()
                {
                    trigger_undo = true;
                }

                ui.separator();

                // Строка быстрого поиска по центру (как в Варианте 4)
                ui.label("🔍");
                let search_hint = self.tr("Поиск по списку...", "Search files...");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(search_hint)
                            .desired_width(170.0),
                    )
                    .changed()
                {
                    needs_recalc = true;
                }
                if !self.search_query.is_empty() && ui.small_button("✖").clicked() {
                    self.search_query.clear();
                    needs_recalc = true;
                }

                // Правый блок: переключатель Темы и Языка
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Переключатель языка RU / EN
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

                    // Переключатель Светлой и Тёмной темы
                    let theme_label = if self.dark_mode {
                        self.tr("🌙 Тёмная", "🌙 Dark")
                    } else {
                        self.tr("☀ Светлая", "☀ Light")
                    };
                    if ui
                        .button(theme_label)
                        .on_hover_text(self.tr("Сменить тему оформления", "Toggle Light/Dark theme"))
                        .clicked()
                    {
                        self.dark_mode = !self.dark_mode;
                        if self.dark_mode {
                            ctx.set_visuals(egui::Visuals::dark());
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        }
                    }

                    ui.separator();

                    if ui
                        .checkbox(&mut self.recursive, self.tr("Вложенные", "Recursive"))
                        .changed()
                    {
                        needs_reload = true;
                    }
                    needs_recalc |= ui
                        .checkbox(&mut self.include_dirs, self.tr("Папки", "Folders"))
                        .changed();
                    needs_recalc |= ui
                        .checkbox(&mut self.include_files, self.tr("Файлы", "Files"))
                        .changed();
                });
            });
            ui.add_space(4.0);
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
                        .unwrap_or_else(|| self.tr("Папка не выбрана", "No folder selected").to_string());
                    ui.weak(path_str);
                });
            });
        });

        // --- ЛЕВАЯ КОЛОНКА: ДЕРЕВО ПАПОК И ПРЕСЕТЫ (Вариант 4) ---
        egui::SidePanel::left("left_explorer_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(170.0..=340.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.heading(self.tr("Папки", "Folders"));
                ui.separator();

                if let Some(curr) = &self.current_dir {
                    if let Some(parent) = curr.parent() {
                        if ui
                            .button(self.tr("⬆ На уровень вверх (..)", "⬆ Up one level (..)"))
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
                    ui.weak(self.tr(
                        "Нажмите «Открыть папку» или перетащите каталог сюда",
                        "Click 'Add Folder' or drag a folder here",
                    ));
                }

                egui::ScrollArea::vertical()
                    .id_salt("folders_scroll")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for sub in &self.subfolders {
                            let name = sub
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            if ui.selectable_label(false, format!("  📁 {}", name)).clicked() {
                                navigate_to = Some(sub.clone());
                            }
                        }
                    });

                ui.add_space(12.0);
                ui.heading(self.tr("Шаблоны правил", "Saved Presets"));
                ui.separator();
                ui.weak(self.tr("Быстрые настройки в 1 клик:", "One-click presets:"));
                ui.add_space(4.0);

                if ui
                    .button(self.tr("🔹 Пробелы → подчёркивания (_)", "🔹 Spaces to underscores (_)"))
                    .clicked()
                {
                    self.find_str = " ".to_string();
                    self.replace_str = "_".to_string();
                    self.use_regex = false;
                    needs_recalc = true;
                }
                if ui
                    .button(self.tr("🔹 Подчёркивания (_) → пробелы", "🔹 Underscores (_) to spaces"))
                    .clicked()
                {
                    self.find_str = "_".to_string();
                    self.replace_str = " ".to_string();
                    self.use_regex = false;
                    needs_recalc = true;
                }
                if ui
                    .button(self.tr("🔹 Нумерация фото (IMG_001)", "🔹 Photo sequence (IMG_001)"))
                    .clicked()
                {
                    self.prefix = "IMG_".to_string();
                    self.num_mode = NumMode::Suffix;
                    self.num_start = 1;
                    self.num_pad = 3;
                    self.num_sep = "_".to_string();
                    needs_recalc = true;
                }
                if ui
                    .button(self.tr("🔹 Строчные имена + счётчик", "🔹 Lowercase + counter"))
                    .clicked()
                {
                    self.case_mode = CaseMode::Lower;
                    self.num_mode = NumMode::Suffix;
                    self.num_start = 1;
                    self.num_pad = 2;
                    self.num_sep = "_".to_string();
                    needs_recalc = true;
                }
                if ui
                    .button(self.tr("🔹 Убрать копии « (1)», « (2)»", "🔹 Remove ' (1)', ' (2)' copies"))
                    .clicked()
                {
                    self.find_str = r"\s*\(\d+\)".to_string();
                    self.replace_str = String::new();
                    self.use_regex = true;
                    needs_recalc = true;
                }

                ui.add_space(8.0);
                if ui
                    .button(self.tr("✖ Сбросить все правила", "✖ Reset All Rules"))
                    .clicked()
                {
                    trigger_reset = true;
                }
            });

        // --- ПРАВАЯ КОЛОНКА: ИНСПЕКТОР ПРАВИЛ ПЕРЕИМЕНОВАНИЯ (Вариант 4) ---
        egui::SidePanel::right("right_rules_panel")
            .resizable(true)
            .default_width(290.0)
            .width_range(250.0..=400.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.heading(self.tr("Правила переименования", "Rename Rules"));
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("rules_scroll")
                    .show(ui, |ui| {
                        // Блок 1: Поиск и замена
                        ui.group(|ui| {
                            ui.strong(self.tr("1. Поиск и замена", "1. Find & Replace"));
                            ui.add_space(2.0);
                            ui.label(self.tr("Найти:", "Find:"));
                            needs_recalc |= ui.text_edit_singleline(&mut self.find_str).changed();
                            ui.label(self.tr("Заменить на:", "Replace with:"));
                            needs_recalc |= ui.text_edit_singleline(&mut self.replace_str).changed();
                            needs_recalc |= ui
                                .checkbox(&mut self.use_regex, self.tr("Регулярные выражения (Regex)", "Use Regular Expressions"))
                                .changed();
                            needs_recalc |= ui
                                .checkbox(&mut self.case_sensitive, self.tr("Учитывать регистр", "Case Sensitive"))
                                .changed();
                        });

                        ui.add_space(6.0);

                        // Блок 2: Трансформация регистра
                        ui.group(|ui| {
                            ui.strong(self.tr("2. Регистр букв", "2. Case Transformation"));
                            ui.add_space(2.0);
                            egui::ComboBox::from_id_salt("case_cb")
                                .width(230.0)
                                .selected_text(match self.case_mode {
                                    CaseMode::None => self.tr("Не менять", "Default (No change)"),
                                    CaseMode::Lower => self.tr("все строчные", "lowercase"),
                                    CaseMode::Upper => self.tr("ВСЕ ПРОПИСНЫЕ", "UPPERCASE"),
                                    CaseMode::Capitalize => self.tr("Первая буква заглавная", "Capitalize first letter"),
                                    CaseMode::Title => self.tr("Каждое Слово С Заглавной", "Title Case Words"),
                                })
                                .show_ui(ui, |ui| {
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.case_mode, CaseMode::None, self.tr("Не менять", "Default (No change)"))
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.case_mode, CaseMode::Lower, self.tr("все строчные", "lowercase"))
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.case_mode, CaseMode::Upper, self.tr("ВСЕ ПРОПИСНЫЕ", "UPPERCASE"))
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.case_mode, CaseMode::Capitalize, self.tr("Первая буква заглавная", "Capitalize first letter"))
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.case_mode, CaseMode::Title, self.tr("Каждое Слово С Заглавной", "Title Case Words"))
                                        .changed();
                                });
                        });

                        ui.add_space(6.0);

                        // Блок 3: Добавление и обрезка
                        ui.group(|ui| {
                            ui.strong(self.tr("3. Префикс, суффикс и обрезка", "3. Add Prefix/Suffix & Trim"));
                            ui.add_space(2.0);
                            egui::Grid::new("prefix_grid").num_columns(2).show(ui, |ui| {
                                ui.label(self.tr("Префикс:", "Prefix:"));
                                needs_recalc |= ui.text_edit_singleline(&mut self.prefix).changed();
                                ui.end_row();

                                ui.label(self.tr("Суффикс:", "Suffix:"));
                                needs_recalc |= ui.text_edit_singleline(&mut self.suffix).changed();
                                ui.end_row();

                                ui.label(self.tr("Срезать слева:", "Trim left:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.trim_start).range(0..=100))
                                    .changed();
                                ui.end_row();

                                ui.label(self.tr("Срезать справа:", "Trim right:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.trim_end).range(0..=100))
                                    .changed();
                                ui.end_row();
                            });
                        });

                        ui.add_space(6.0);

                        // Блок 4: Автонумерация
                        ui.group(|ui| {
                            ui.strong(self.tr("4. Нумерация (Сериализация)", "4. Serialization (Numbering)"));
                            ui.add_space(2.0);
                            egui::ComboBox::from_id_salt("num_cb")
                                .width(230.0)
                                .selected_text(match self.num_mode {
                                    NumMode::None => self.tr("Выключена", "Disabled"),
                                    NumMode::Suffix => self.tr("В конец имени (Суффикс)", "Append to end (Suffix)"),
                                    NumMode::Prefix => self.tr("В начало имени (Префикс)", "Prepend to start (Prefix)"),
                                })
                                .show_ui(ui, |ui| {
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.num_mode, NumMode::None, self.tr("Выключена", "Disabled"))
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.num_mode, NumMode::Suffix, self.tr("В конец имени (Суффикс)", "Append to end (Suffix)"))
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(&mut self.num_mode, NumMode::Prefix, self.tr("В начало имени (Префикс)", "Prepend to start (Prefix)"))
                                        .changed();
                                });

                            ui.horizontal(|ui| {
                                ui.label(self.tr("Старт:", "Start:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_start).range(0..=99999))
                                    .changed();
                                ui.label(self.tr("Шаг:", "Step:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_step).range(1..=1000))
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(self.tr("Цифр (001):", "Zero pad:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_pad).range(1..=10))
                                    .changed();
                                ui.label(self.tr("Раздел.:", "Sep:"));
                                needs_recalc |= ui
                                    .add(egui::TextEdit::singleline(&mut self.num_sep).desired_width(36.0))
                                    .changed();
                            });

                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.reset_num_per_dir,
                                    self.tr("Сброс счётчика в подпапках", "Reset counter per folder"),
                                )
                                .changed();
                        });

                        ui.add_space(6.0);

                        // Блок 5: Фильтр по расширениям
                        ui.group(|ui| {
                            ui.strong(self.tr("5. Фильтр расширений", "5. Extension Filter"));
                            ui.label(self.tr("Только расширения (jpg, png):", "Only extensions (jpg, png):"));
                            needs_recalc |= ui.text_edit_singleline(&mut self.ext_filter).changed();
                        });

                        ui.add_space(12.0);

                        // Главная синяя кнопка «Применить изменения» (как в Варианте 4)
                        let btn_text = egui::RichText::new(
                            self.tr("✔ Применить изменения", "✔ Apply Changes"),
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

        // --- ЦЕНТРАЛЬНАЯ ТАБЛИЦА ПРЕДПРОСМОТРА (Вариант 4) ---
        egui::CentralPanel::default().show(ctx, |ui| {
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

            ui.heading(self.tr("Файлы и папки для переименования", "Files to Rename"));
            ui.add_space(4.0);

            let mut toggle_all: Option<bool> = None;
            let mut toggled_item: Option<(PathBuf, bool)> = None;
            let all_included = !self.preview_rows.is_empty()
                && self.preview_rows.iter().all(|r| r.included);

            let col_type_title = self.tr("Тип", "Type");
            let col_orig_title = self.tr("Текущее имя", "Original Name");
            let col_new_title = self.tr("Новое имя", "New Name");
            let col_status_title = self.tr("Статус", "Status");
            let col_size_title = self.tr("Размер", "Size");

            let status_ready = self.tr("Готов", "Ready");
            let status_changed = self.tr("Изменится", "Preview");
            let status_skipped = self.tr("Пропущен", "Skipped");
            let label_folder = self.tr("📁 Папка", "📁 Folder");
            let label_file = self.tr("📄 Файл", "📄 File");

            let is_dark = self.dark_mode;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(32.0))
                .column(Column::initial(95.0).resizable(true))
                .column(Column::initial(240.0).resizable(true))
                .column(Column::initial(240.0).resizable(true))
                .column(Column::initial(90.0).resizable(true))
                .column(Column::remainder())
                .header(24.0, |mut header| {
                    header.col(|ui| {
                        let mut master = all_included;
                        if ui.checkbox(&mut master, "").changed() {
                            toggle_all = Some(master);
                        }
                    });
                    header.col(|ui| {
                        ui.strong(col_type_title);
                    });
                    header.col(|ui| {
                        ui.strong(col_orig_title);
                    });
                    header.col(|ui| {
                        ui.strong(col_new_title);
                    });
                    header.col(|ui| {
                        ui.strong(col_status_title);
                    });
                    header.col(|ui| {
                        ui.strong(col_size_title);
                    });
                })
                .body(|body| {
                    body.rows(24.0, self.preview_rows.len(), |mut row| {
                        let item = &self.preview_rows[row.index()];
                        let muted = egui::Color32::from_gray(130);

                        row.col(|ui| {
                            let mut inc = item.included;
                            if ui.checkbox(&mut inc, "").changed() {
                                toggled_item = Some((item.old_path.clone(), inc));
                            }
                        });

                        row.col(|ui| {
                            let kind_str = if item.is_dir { label_folder } else { label_file };
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
                            if item.included {
                                ui.label(display_orig);
                            } else {
                                ui.colored_label(muted, display_orig);
                            }
                        });

                        // Столбец «Новое имя» с цветной плашкой (как в Варианте 4)
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
                    });
                });

            if let Some(select_all) = toggle_all {
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
