#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use regex::RegexBuilder;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ReStem — Пакетное переименование файлов и папок")
            .with_inner_size([1150.0, 720.0])
            .with_min_inner_size([900.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ReStem — Пакетное переименование файлов и папок",
        options,
        Box::new(|_cc| Ok(Box::new(ReStemApp::default()))),
    )
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
}

#[derive(Clone)]
struct PreviewRow {
    old_path: PathBuf,
    parent_dir: PathBuf,
    rel_parent: String,
    is_dir: bool,
    depth: usize,
    old_name: String,
    new_name: String,
    changed: bool,
}

struct ReStemApp {
    current_dir: Option<PathBuf>,
    all_entries: Vec<FsEntry>,
    preview_rows: Vec<PreviewRow>,
    undo_history: Vec<(PathBuf, PathBuf)>,

    // Настройки выбора объектов
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
            current_dir: None,
            all_entries: Vec::new(),
            preview_rows: Vec::new(),
            undo_history: Vec::new(),
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
            status_msg: "ReStem готов к работе. Выберите корневую папку.".to_string(),
        }
    }
}

impl ReStemApp {
    fn load_directory(&mut self, root_dir: PathBuf) {
        let mut entries = Vec::new();
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

        let mut local_entries: Vec<PathBuf> = read_dir
            .flatten()
            .map(|e| e.path())
            .collect();

        local_entries.sort_by_key(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        });

        for path in local_entries {
            let is_dir = path.is_dir();
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
            });

            if is_dir && self.recursive {
                self.collect_entries(root, &path, depth + 1, out);
            }
        }
    }

    fn compute_new_name(&self, entry: &FsEntry, idx: usize) -> String {
        let (mut stem, ext) = if entry.is_dir {
            // Для папок точки в названии не считаются расширением файла
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

        // 1. Обрезка символов по краям (посимвольно с поддержкой UTF-8)
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
            let new_name = self.compute_new_name(entry, counter_idx);
            counter_idx += 1;

            let changed = old_name != new_name && !new_name.is_empty();
            if changed {
                changed_count += 1;
            }

            self.preview_rows.push(PreviewRow {
                old_path: entry.path.clone(),
                parent_dir: entry.parent_dir.clone(),
                rel_parent: entry.rel_parent.clone(),
                is_dir: entry.is_dir,
                depth: entry.depth,
                old_name,
                new_name,
                changed,
            });
        }

        self.status_msg = format!(
            "Объектов в списке: {} | Будет переименовано: {}",
            self.preview_rows.len(),
            changed_count
        );
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
        // Проверка дубликатов внутри одной и той же родительской папки
        let mut seen: HashSet<(PathBuf, String)> = HashSet::new();
        for row in &self.preview_rows {
            let key = (row.parent_dir.clone(), row.new_name.to_lowercase());
            if !seen.insert(key) {
                self.status_msg = format!(
                    "Ошибка: конфликт имени '{}' в папке '{}'",
                    row.new_name, row.rel_parent
                );
                return;
            }
        }

        // Собираем только изменяемые элементы и сортируем снизу вверх (от глубоких к корню)
        let mut to_rename: Vec<PreviewRow> = self
            .preview_rows
            .iter()
            .filter(|r| r.changed)
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
                self.status_msg = format!(
                    "Ошибка: '{}' уже существует в '{}'",
                    row.new_name, row.rel_parent
                );
                break;
            }

            if let Err(e) = fs::rename(&row.old_path, &new_path) {
                self.status_msg = format!("Ошибка переименования {}: {}", row.old_name, e);
                break;
            }
            history.push((new_path, row.old_path));
        }

        let count = history.len();
        self.undo_history = history;
        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir);
        }
        self.status_msg = format!("Успешно переименовано объектов: {}", count);
    }

    fn undo_rename(&mut self) {
        let mut restored = 0;
        // Откат выполняется в обратном порядке (сверху вниз: от родительских папок к вложенным файлам)
        for (curr, orig) in self.undo_history.drain(..).rev() {
            if fs::rename(&curr, &orig).is_ok() {
                restored += 1;
            }
        }
        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir);
        }
        self.status_msg = format!("Откат выполнен: возвращено {} имён", restored);
    }
}

impl eframe::App for ReStemApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut needs_recalc = false;
        let mut needs_reload = false;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Обзор папки...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.current_dir = Some(folder);
                        needs_reload = true;
                    }
                }
                let dir_text = self
                    .current_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Папка не выбрана".to_string());
                ui.label(dir_text);
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.strong("Объекты:");
                needs_recalc |= ui.checkbox(&mut self.include_files, "Файлы").changed();
                needs_recalc |= ui.checkbox(&mut self.include_dirs, "Папки").changed();
                if ui
                    .checkbox(&mut self.recursive, "Вложенные (рекурсивно)")
                    .changed()
                {
                    needs_reload = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.ext_filter).desired_width(120.0))
                        .changed()
                    {
                        needs_recalc = true;
                    }
                    ui.label("Фильтр расширений файлов (jpg, png):");
                });
            });
            ui.add_space(2.0);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(&self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Переименовать").clicked() {
                        self.apply_rename();
                    }
                    if ui
                        .add_enabled(
                            !self.undo_history.is_empty(),
                            egui::Button::new("Undo (Откат)"),
                        )
                        .clicked()
                    {
                        self.undo_rename();
                    }
                    if ui.button("Сбросить правила").clicked() {
                        self.reset_rules();
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(3, |cols| {
                cols[0].group(|ui| {
                    ui.strong("1. Поиск и замена");
                    egui::Grid::new("grid_replace").num_columns(2).show(ui, |ui| {
                        ui.label("Найти:");
                        needs_recalc |= ui.text_edit_singleline(&mut self.find_str).changed();
                        ui.end_row();
                        ui.label("Заменить:");
                        needs_recalc |= ui.text_edit_singleline(&mut self.replace_str).changed();
                        ui.end_row();
                    });
                    needs_recalc |= ui
                        .checkbox(&mut self.use_regex, "Регулярные выражения (Regex)")
                        .changed();
                    needs_recalc |= ui
                        .checkbox(&mut self.case_sensitive, "Учитывать регистр")
                        .changed();
                });

                cols[1].group(|ui| {
                    ui.strong("2. Добавление и обрезка");
                    egui::Grid::new("grid_add").num_columns(2).show(ui, |ui| {
                        ui.label("Префикс:");
                        needs_recalc |= ui.text_edit_singleline(&mut self.prefix).changed();
                        ui.end_row();
                        ui.label("Суффикс:");
                        needs_recalc |= ui.text_edit_singleline(&mut self.suffix).changed();
                        ui.end_row();
                        ui.label("Удалить слева:");
                        needs_recalc |= ui
                            .add(egui::DragValue::new(&mut self.trim_start).range(0..=100))
                            .changed();
                        ui.end_row();
                        ui.label("Удалить справа:");
                        needs_recalc |= ui
                            .add(egui::DragValue::new(&mut self.trim_end).range(0..=100))
                            .changed();
                        ui.end_row();
                    });
                });

                cols[2].group(|ui| {
                    ui.strong("3. Регистр и нумерация");
                    ui.horizontal(|ui| {
                        ui.label("Регистр:");
                        egui::ComboBox::from_id_salt("case_cb")
                            .selected_text(match self.case_mode {
                                CaseMode::None => "Не менять",
                                CaseMode::Lower => "строчные",
                                CaseMode::Upper => "ПРОПИСНЫЕ",
                                CaseMode::Capitalize => "Первая заглавная",
                                CaseMode::Title => "Каждое Слово",
                            })
                            .show_ui(ui, |ui| {
                                needs_recalc |= ui
                                    .selectable_value(&mut self.case_mode, CaseMode::None, "Не менять")
                                    .changed();
                                needs_recalc |= ui
                                    .selectable_value(&mut self.case_mode, CaseMode::Lower, "строчные")
                                    .changed();
                                needs_recalc |= ui
                                    .selectable_value(&mut self.case_mode, CaseMode::Upper, "ПРОПИСНЫЕ")
                                    .changed();
                                needs_recalc |= ui
                                    .selectable_value(
                                        &mut self.case_mode,
                                        CaseMode::Capitalize,
                                        "Первая заглавная",
                                    )
                                    .changed();
                                needs_recalc |= ui
                                    .selectable_value(
                                        &mut self.case_mode,
                                        CaseMode::Title,
                                        "Каждое Слово",
                                    )
                                    .changed();
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Нумерация:");
                        egui::ComboBox::from_id_salt("num_cb")
                            .selected_text(match self.num_mode {
                                NumMode::None => "Нет",
                                NumMode::Suffix => "В конец",
                                NumMode::Prefix => "В начало",
                            })
                            .show_ui(ui, |ui| {
                                needs_recalc |= ui
                                    .selectable_value(&mut self.num_mode, NumMode::None, "Нет")
                                    .changed();
                                needs_recalc |= ui
                                    .selectable_value(&mut self.num_mode, NumMode::Suffix, "В конец")
                                    .changed();
                                needs_recalc |= ui
                                    .selectable_value(&mut self.num_mode, NumMode::Prefix, "В начало")
                                    .changed();
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Старт:");
                        needs_recalc |= ui
                            .add(egui::DragValue::new(&mut self.num_start).range(0..=99999))
                            .changed();
                        ui.label("Шаг:");
                        needs_recalc |= ui
                            .add(egui::DragValue::new(&mut self.num_step).range(1..=1000))
                            .changed();
                        ui.label("Нулей:");
                        needs_recalc |= ui
                            .add(egui::DragValue::new(&mut self.num_pad).range(1..=10))
                            .changed();
                        ui.label("Раздел.:");
                        needs_recalc |= ui
                            .add(egui::TextEdit::singleline(&mut self.num_sep).desired_width(30.0))
                            .changed();
                    });

                    needs_recalc |= ui
                        .checkbox(&mut self.reset_num_per_dir, "Сброс счётчика в каждой папке")
                        .changed();
                });
            });

            if needs_reload {
                if let Some(dir) = self.current_dir.clone() {
                    self.load_directory(dir);
                }
            } else if needs_recalc {
                self.recalculate_preview();
            }

            ui.separator();

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(70.0).resizable(true))
                .column(Column::initial(220.0).resizable(true))
                .column(Column::initial(320.0).resizable(true))
                .column(Column::initial(320.0).resizable(true))
                .column(Column::remainder())
                .header(22.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Тип");
                    });
                    header.col(|ui| {
                        ui.strong("Расположение");
                    });
                    header.col(|ui| {
                        ui.strong("Текущее имя");
                    });
                    header.col(|ui| {
                        ui.strong("Новое имя (предпросмотр)");
                    });
                    header.col(|ui| {
                        ui.strong("Статус");
                    });
                })
                .body(|body| {
                    body.rows(20.0, self.preview_rows.len(), |mut row| {
                        let item = &self.preview_rows[row.index()];
                        row.col(|ui| {
                            if item.is_dir {
                                ui.colored_label(egui::Color32::from_rgb(230, 160, 40), "[Папка]");
                            } else {
                                ui.label("Файл");
                            }
                        });
                        row.col(|ui| {
                            ui.label(&item.rel_parent);
                        });
                        row.col(|ui| {
                            ui.label(&item.old_name);
                        });
                        row.col(|ui| {
                            if item.changed {
                                ui.colored_label(
                                    egui::Color32::from_rgb(40, 180, 99),
                                    &item.new_name,
                                );
                            } else {
                                ui.label(&item.new_name);
                            }
                        });
                        row.col(|ui| {
                            if item.changed {
                                ui.colored_label(egui::Color32::from_rgb(40, 180, 99), "Изменится");
                            } else {
                                ui.label("Без изменений");
                            }
                        });
                    });
                });
        });
    }
}
