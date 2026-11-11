#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use regex::RegexBuilder;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ReStem — Пакетное переименование")
            .with_inner_size([1050.0, 700.0])
            .with_min_inner_size([850.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ReStem — Пакетное переименование",
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

struct PreviewRow {
    old_path: PathBuf,
    old_name: String,
    new_name: String,
    changed: bool,
}

struct ReStemApp {
    current_dir: Option<PathBuf>,
    all_files: Vec<PathBuf>,
    preview_rows: Vec<PreviewRow>,
    undo_history: Vec<(PathBuf, PathBuf)>,

    ext_filter: String,
    find_str: String,
    replace_str: String,
    use_regex: bool,
    case_sensitive: bool,

    prefix: String,
    suffix: String,
    trim_start: usize,
    trim_end: usize,

    case_mode: CaseMode,
    num_mode: NumMode,
    num_start: usize,
    num_step: usize,
    num_pad: usize,
    num_sep: String,

    status_msg: String,
}

impl Default for ReStemApp {
    fn default() -> Self {
        Self {
            current_dir: None,
            all_files: Vec::new(),
            preview_rows: Vec::new(),
            undo_history: Vec::new(),
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
            status_msg: "ReStem готов к работе. Выберите папку с файлами.".to_string(),
        }
    }
}

impl ReStemApp {
    fn load_directory(&mut self, dir: PathBuf) {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    files.push(path);
                }
            }
        }
        files.sort_by_key(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        });
        self.current_dir = Some(dir);
        self.all_files = files;
        self.recalculate_preview();
    }

    fn compute_new_name(&self, path: &PathBuf, idx: usize) -> String {
        let mut stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();

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

        let filtered: Vec<&PathBuf> = self
            .all_files
            .iter()
            .filter(|p| {
                if allowed_exts.is_empty() {
                    return true;
                }
                let ext = p
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                allowed_exts.contains(&ext)
            })
            .collect();

        self.preview_rows.clear();
        let mut changed_count = 0;

        for (idx, path) in filtered.into_iter().enumerate() {
            let old_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let new_name = self.compute_new_name(path, idx);
            let changed = old_name != new_name;
            if changed {
                changed_count += 1;
            }
            self.preview_rows.push(PreviewRow {
                old_path: path.clone(),
                old_name,
                new_name,
                changed,
            });
        }

        self.status_msg = format!(
            "Файлов в списке: {} | Будет переименовано: {}",
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
        let mut seen = HashSet::new();
        for row in &self.preview_rows {
            let lower = row.new_name.to_lowercase();
            if !seen.insert(lower) {
                self.status_msg = format!("Ошибка: дубликат имени '{}'", row.new_name);
                return;
            }
        }

        let mut history = Vec::new();
        for row in &self.preview_rows {
            if row.changed {
                let new_path = row.old_path.with_file_name(&row.new_name);
                if let Err(e) = fs::rename(&row.old_path, &new_path) {
                    self.status_msg = format!("Ошибка переименования {}: {}", row.old_name, e);
                    break;
                }
                history.push((new_path, row.old_path.clone()));
            }
        }

        let count = history.len();
        self.undo_history = history;
        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir);
        }
        self.status_msg = format!("Успешно переименовано файлов: {}", count);
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
        self.status_msg = format!("Откат выполнен: возвращено {} имён", restored);
    }
}

impl eframe::App for ReStemApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut needs_recalc = false;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Обзор папки...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.load_directory(folder);
                    }
                }
                let dir_text = self
                    .current_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Папка не выбрана".to_string());
                ui.label(dir_text);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.ext_filter).desired_width(120.0))
                        .changed()
                    {
                        needs_recalc = true;
                    }
                    ui.label("Фильтр расширений (jpg, png):");
                });
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(&self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Переименовать файлы").clicked() {
                        self.apply_rename();
                    }
                    if ui
                        .add_enabled(!self.undo_history.is_empty(), egui::Button::new("Undo (Откат)"))
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
                    needs_recalc |= ui.checkbox(&mut self.use_regex, "Регулярные выражения (Regex)").changed();
                    needs_recalc |= ui.checkbox(&mut self.case_sensitive, "Учитывать регистр").changed();
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
                        needs_recalc |= ui.add(egui::DragValue::new(&mut self.trim_start).range(0..=100)).changed();
                        ui.end_row();
                        ui.label("Удалить справа:");
                        needs_recalc |= ui.add(egui::DragValue::new(&mut self.trim_end).range(0..=100)).changed();
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
                                needs_recalc |= ui.selectable_value(&mut self.case_mode, CaseMode::None, "Не менять").changed();
                                needs_recalc |= ui.selectable_value(&mut self.case_mode, CaseMode::Lower, "строчные").changed();
                                needs_recalc |= ui.selectable_value(&mut self.case_mode, CaseMode::Upper, "ПРОПИСНЫЕ").changed();
                                needs_recalc |= ui.selectable_value(&mut self.case_mode, CaseMode::Capitalize, "Первая заглавная").changed();
                                needs_recalc |= ui.selectable_value(&mut self.case_mode, CaseMode::Title, "Каждое Слово").changed();
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
                                needs_recalc |= ui.selectable_value(&mut self.num_mode, NumMode::None, "Нет").changed();
                                needs_recalc |= ui.selectable_value(&mut self.num_mode, NumMode::Suffix, "В конец").changed();
                                needs_recalc |= ui.selectable_value(&mut self.num_mode, NumMode::Prefix, "В начало").changed();
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Старт:");
                        needs_recalc |= ui.add(egui::DragValue::new(&mut self.num_start).range(0..=99999)).changed();
                        ui.label("Шаг:");
                        needs_recalc |= ui.add(egui::DragValue::new(&mut self.num_step).range(1..=1000)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Нулей:");
                        needs_recalc |= ui.add(egui::DragValue::new(&mut self.num_pad).range(1..=10)).changed();
                        ui.label("Раздел.:");
                        needs_recalc |= ui.add(egui::TextEdit::singleline(&mut self.num_sep).desired_width(40.0)).changed();
                    });
                });
            });

            if needs_recalc {
                self.recalculate_preview();
            }

            ui.separator();

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(380.0).resizable(true))
                .column(Column::initial(380.0).resizable(true))
                .column(Column::remainder())
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("Текущее имя файла"); });
                    header.col(|ui| { ui.strong("Новое имя (предпросмотр)"); });
                    header.col(|ui| { ui.strong("Статус"); });
                })
                .body(|body| {
                    body.rows(20.0, self.preview_rows.len(), |mut row| {
                        let item = &self.preview_rows[row.index()];
                        row.col(|ui| { ui.label(&item.old_name); });
                        row.col(|ui| {
                            if item.changed {
                                ui.colored_label(egui::Color32::from_rgb(40, 180, 99), &item.new_name);
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
