#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_info;
mod cli;
mod hash_utils;
mod media_info;
mod photo_meta;
mod rename_engine;
mod scanner;
mod updater;

pub(crate) use rename_engine::decompose_timestamp;

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use hash_utils::{DupScanMsg, DuplicateGroup};
use rename_engine::{
    build_rollback_bat, compute_preview, default_rule_order, execute_rename_batch,
    execute_undo_batch, format_timestamp, load_presets_json, save_presets_json, CaseMode, ExtMode,
    FsEntry, FullPreset, HashCache, Lang, NumMode, PreviewRow, RenameConfig, ReplaceStep,
    RowIssue, RuleStage, SizeFilterMode, SortField, StatusFilter, TranslitMode, UndoBatch,
};
use scanner::{ScanJob, ScanMsg};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use updater::{UpdateCheckState, UpdateInfo};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_LICENSE: &str = match option_env!("CARGO_PKG_LICENSE") {
    Some(lic) if !lic.is_empty() => lic,
    _ => "GPL-3.0-only",
};
const GITHUB_REPO_URL: &str = match option_env!("CARGO_PKG_REPOSITORY") {
    Some(url) if !url.is_empty() => url,
    _ => "https://github.com/pirotehnikng-cloud/ReStem",
};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();
    let cli_dir = match cli::handle_cli_args(&args, APP_VERSION) {
        cli::CliAction::ExitCode(code) => std::process::exit(code),
        cli::CliAction::LaunchGui(dir) => dir,
    };

    let icon = build_app_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!(
                "ReStem v{} — Пакетное переименование файлов и папок",
                APP_VERSION
            ))
            .with_inner_size([1420.0, 860.0])
            .with_min_inner_size([960.0, 600.0])
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
            if app.check_updates {
                app.trigger_update_check(&cc.egui_ctx);
            }
            if let Some(dir) = cli_dir {
                app.open_new_directory(dir, &cc.egui_ctx);
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

    style.spacing.item_spacing = egui::vec2(7.0, 5.5);
    style.spacing.button_padding = egui::vec2(7.5, 4.0);

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

fn texture_from_memory(
    ctx: &egui::Context,
    id: &str,
    bytes: &[u8],
    max_dim: u32,
) -> Option<(egui::TextureHandle, egui::Vec2)> {
    let img = image::load_from_memory(bytes).ok()?;
    let thumb = img.thumbnail(max_dim, max_dim);
    let rgba = thumb.to_rgba8();
    let tw = rgba.width() as usize;
    let th = rgba.height() as usize;
    let color_img =
        egui::ColorImage::from_rgba_unmultiplied([tw, th], rgba.as_flat_samples().as_slice());
    let tex = ctx.load_texture(id, color_img, egui::TextureOptions::LINEAR);
    Some((tex, egui::vec2(tw as f32, th as f32)))
}

fn open_in_system(path: &Path) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer").arg(path).spawn();
    }
    #[cfg(not(windows))]
    let _ = path;
}

fn open_url_in_browser(url: &str) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer").arg(url).spawn();
    }
    #[cfg(not(windows))]
    let _ = url;
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

enum PreviewState {
    Empty,
    Folder {
        name: String,
        items_count: usize,
    },
    Image {
        name: String,
        texture: egui::TextureHandle,
        meta: photo_meta::PhotoMeta,
        tex_size: egui::Vec2,
    },
    PhotoMetaOnly {
        name: String,
        ext_upper: String,
        meta: photo_meta::PhotoMeta,
    },
    Video {
        name: String,
        info: media_info::VideoMediaInfo,
        size_bytes: u64,
        thumb: Option<(egui::TextureHandle, egui::Vec2)>,
        loading_thumb: bool,
    },
    Audio {
        name: String,
        info: audio_info::AudioMediaInfo,
        size_bytes: u64,
        cover: Option<(egui::TextureHandle, egui::Vec2)>,
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

struct ReStemApp {
    lang: Lang,
    dark_mode: bool,
    ui_scale: f32,
    inline_diff: bool,
    use_external_mediainfo: bool,
    check_updates: bool,

    current_dir: Option<PathBuf>,
    subfolders: Vec<PathBuf>,
    all_entries: Vec<FsEntry>,
    preview_rows: Vec<PreviewRow>,
    excluded_paths: HashSet<PathBuf>,
    manual_overrides: HashMap<PathBuf, String>,
    hash_cache: HashCache,

    undo_stack: Vec<UndoBatch>,

    scan_job: Option<ScanJob>,
    video_thumb_rx: Option<mpsc::Receiver<(PathBuf, Option<Vec<u8>>)>>,

    selected_path: Option<PathBuf>,
    preview_state: PreviewState,

    editing_row_path: Option<PathBuf>,
    editing_row_text: String,

    confirm_trash_modal: bool,
    show_about_modal: bool,
    show_regex_help_modal: bool,
    show_list_rename_modal: bool,
    show_duplicates_modal: bool,

    dup_scanning: bool,
    dup_progress: (usize, usize),
    dup_cancel: Option<Arc<AtomicBool>>,
    dup_rx: Option<mpsc::Receiver<DupScanMsg>>,
    dup_groups: Vec<DuplicateGroup>,
    dup_checked: HashSet<PathBuf>,

    update_state: UpdateCheckState,
    update_rx: Option<mpsc::Receiver<Result<Option<UpdateInfo>, String>>>,

    user_presets: Vec<FullPreset>,
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
    extra_replaces: Vec<ReplaceStep>,
    rule_order: Vec<RuleStage>,

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

    auto_suffix_duplicates: bool,
    sync_sidecars: bool,
    organize_enabled: bool,
    organize_template: String,

    issue_count: usize,
    status_msg: String,
}

impl Default for ReStemApp {
    fn default() -> Self {
        Self {
            lang: Lang::Ru,
            dark_mode: false,
            ui_scale: 1.0,
            inline_diff: true,
            use_external_mediainfo: true,
            check_updates: true,

            current_dir: None,
            subfolders: Vec::new(),
            all_entries: Vec::new(),
            preview_rows: Vec::new(),
            excluded_paths: HashSet::new(),
            manual_overrides: HashMap::new(),
            hash_cache: HashCache::new(),
            undo_stack: Vec::new(),

            scan_job: None,
            video_thumb_rx: None,

            selected_path: None,
            preview_state: PreviewState::Empty,

            editing_row_path: None,
            editing_row_text: String::new(),

            confirm_trash_modal: false,
            show_about_modal: false,
            show_regex_help_modal: false,
            show_list_rename_modal: false,
            show_duplicates_modal: false,

            dup_scanning: false,
            dup_progress: (0, 0),
            dup_cancel: None,
            dup_rx: None,
            dup_groups: Vec::new(),
            dup_checked: HashSet::new(),

            update_state: UpdateCheckState::Idle,
            update_rx: None,

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
            extra_replaces: Vec::new(),
            rule_order: default_rule_order(),

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

            auto_suffix_duplicates: false,
            sync_sidecars: false,
            organize_enabled: false,
            organize_template: String::new(),

            issue_count: 0,
            status_msg: "Выберите папку или перетащите её мышью в окно программы.".to_string(),
        }
    }
}

impl ReStemApp {
    fn to_rename_config(&self) -> RenameConfig {
        RenameConfig {
            sort_field: self.sort_field,
            sort_asc: self.sort_asc,
            folders_first: self.folders_first,
            search_query: self.search_query.clone(),
            search_regex: self.search_regex,
            search_case_sensitive: self.search_case_sensitive,
            status_filter: self.status_filter,
            include_files: self.include_files,
            include_dirs: self.include_dirs,
            hide_system_hidden: self.hide_system_hidden,
            ext_filter: self.ext_filter.clone(),
            ext_exclude_filter: self.ext_exclude_filter.clone(),
            exclude_words_filter: self.exclude_words_filter.clone(),
            size_filter_mode: self.size_filter_mode,
            size_min_kb: self.size_min_kb,
            size_max_kb: self.size_max_kb,
            find_str: self.find_str.clone(),
            replace_str: self.replace_str.clone(),
            use_regex: self.use_regex,
            case_sensitive: self.case_sensitive,
            extra_replaces: self.extra_replaces.clone(),
            rule_order: self.rule_order.clone(),
            case_mode: self.case_mode,
            translit_mode: self.translit_mode,
            sanitize_names: self.sanitize_names,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            trim_start: self.trim_start,
            trim_end: self.trim_end,
            insert_text: self.insert_text.clone(),
            insert_pos: self.insert_pos,
            insert_from_end: self.insert_from_end,
            use_custom_list: self.use_custom_list,
            custom_list_raw: self.custom_list_raw.clone(),
            num_mode: self.num_mode,
            num_start: self.num_start,
            num_step: self.num_step,
            num_pad: self.num_pad,
            num_sep: self.num_sep.clone(),
            reset_num_per_dir: self.reset_num_per_dir,
            ext_mode: self.ext_mode,
            new_ext: self.new_ext.clone(),
            auto_suffix_duplicates: self.auto_suffix_duplicates,
            sync_sidecars: self.sync_sidecars,
            organize_enabled: self.organize_enabled,
            organize_template: self.organize_template.clone(),
        }
    }

    fn capture_full_preset(&self, title: String) -> FullPreset {
        FullPreset {
            title,
            find_str: self.find_str.clone(),
            replace_str: self.replace_str.clone(),
            use_regex: self.use_regex,
            case_sensitive: self.case_sensitive,
            extra_replaces: self.extra_replaces.clone(),
            rule_order: self.rule_order.clone(),
            case_mode: self.case_mode,
            translit_mode: self.translit_mode,
            sanitize_names: self.sanitize_names,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            trim_start: self.trim_start,
            trim_end: self.trim_end,
            insert_text: self.insert_text.clone(),
            insert_pos: self.insert_pos,
            insert_from_end: self.insert_from_end,
            num_mode: self.num_mode,
            num_start: self.num_start,
            num_step: self.num_step,
            num_pad: self.num_pad,
            num_sep: self.num_sep.clone(),
            reset_num_per_dir: self.reset_num_per_dir,
            ext_mode: self.ext_mode,
            new_ext: self.new_ext.clone(),
            ext_filter: self.ext_filter.clone(),
            ext_exclude_filter: self.ext_exclude_filter.clone(),
            exclude_words_filter: self.exclude_words_filter.clone(),
            size_filter_mode: self.size_filter_mode,
            size_min_kb: self.size_min_kb,
            size_max_kb: self.size_max_kb,
            auto_suffix_duplicates: self.auto_suffix_duplicates,
            sync_sidecars: self.sync_sidecars,
            organize_enabled: self.organize_enabled,
            organize_template: self.organize_template.clone(),
        }
    }

    fn apply_full_preset(&mut self, p: &FullPreset) {
        self.find_str = p.find_str.clone();
        self.replace_str = p.replace_str.clone();
        self.use_regex = p.use_regex;
        self.case_sensitive = p.case_sensitive;
        self.extra_replaces = p.extra_replaces.clone();
        self.rule_order = if p.rule_order.is_empty() {
            default_rule_order()
        } else {
            p.rule_order.clone()
        };
        self.case_mode = p.case_mode;
        self.translit_mode = p.translit_mode;
        self.sanitize_names = p.sanitize_names;
        self.prefix = p.prefix.clone();
        self.suffix = p.suffix.clone();
        self.trim_start = p.trim_start;
        self.trim_end = p.trim_end;
        self.insert_text = p.insert_text.clone();
        self.insert_pos = p.insert_pos;
        self.insert_from_end = p.insert_from_end;
        self.num_mode = p.num_mode;
        self.num_start = p.num_start;
        self.num_step = p.num_step;
        self.num_pad = p.num_pad;
        self.num_sep = p.num_sep.clone();
        self.reset_num_per_dir = p.reset_num_per_dir;
        self.ext_mode = p.ext_mode;
        self.new_ext = p.new_ext.clone();
        self.ext_filter = p.ext_filter.clone();
        self.ext_exclude_filter = p.ext_exclude_filter.clone();
        self.exclude_words_filter = p.exclude_words_filter.clone();
        self.size_filter_mode = p.size_filter_mode;
        self.size_min_kb = p.size_min_kb;
        self.size_max_kb = p.size_max_kb;
        self.auto_suffix_duplicates = p.auto_suffix_duplicates;
        self.sync_sidecars = p.sync_sidecars;
        self.organize_enabled = p.organize_enabled;
        self.organize_template = p.organize_template.clone();
    }

    fn save_settings(&self) {
        let lines = vec![
            format!("lang={}", if self.lang == Lang::Ru { "ru" } else { "en" }),
            format!("dark_mode={}", self.dark_mode),
            format!("ui_scale={:.2}", self.ui_scale),
            format!("inline_diff={}", self.inline_diff),
            format!("use_external_mediainfo={}", self.use_external_mediainfo),
            format!("check_updates={}", self.check_updates),
        ];
        let _ = fs::write(settings_file_path(), lines.join("\n"));
        save_presets_json(&self.user_presets);
    }

    fn load_settings(&mut self) {
        self.user_presets = load_presets_json();

        let Ok(content) = fs::read_to_string(settings_file_path()) else {
            return;
        };
        let mut migrated = false;
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
            } else if let Some(val) = line.strip_prefix("use_external_mediainfo=") {
                self.use_external_mediainfo = val.trim() != "false";
            } else if let Some(val) = line.strip_prefix("check_updates=") {
                self.check_updates = val.trim() != "false";
            } else if let Some(val) = line.strip_prefix("preset=") {
                if self.user_presets.is_empty() {
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
                        let mut fp = FullPreset::default();
                        fp.title = parts[0].to_string();
                        fp.find_str = parts[1].to_string();
                        fp.replace_str = parts[2].to_string();
                        fp.use_regex = parts[3] == "true";
                        fp.prefix = parts[4].to_string();
                        fp.suffix = parts[5].to_string();
                        fp.translit_mode = translit_mode;
                        fp.case_mode = case_mode;
                        fp.sanitize_names = parts[8] == "true";
                        self.user_presets.push(fp);
                        migrated = true;
                    }
                }
            }
        }
        if migrated {
            save_presets_json(&self.user_presets);
        }
    }

    fn export_presets_json_dialog(&mut self) {
        if let Some(save_path) = rfd::FileDialog::new()
            .set_file_name("restem_presets.json")
            .add_filter("JSON Presets (.json)", &["json"])
            .save_file()
        {
            if let Ok(json) = serde_json::to_string_pretty(&self.user_presets) {
                if fs::write(&save_path, json).is_ok() {
                    self.status_msg = match self.lang {
                        Lang::Ru => format!("Пресеты сохранены: {}", save_path.display()),
                        Lang::En => format!("Presets exported: {}", save_path.display()),
                    };
                }
            }
        }
    }

    fn import_presets_json_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Presets (.json)", &["json"])
            .pick_file()
        {
            if let Ok(raw) = fs::read_to_string(&path) {
                let imported: Vec<FullPreset> =
                    if let Ok(list) = serde_json::from_str::<Vec<FullPreset>>(&raw) {
                        list
                    } else if let Ok(single) = serde_json::from_str::<FullPreset>(&raw) {
                        vec![single]
                    } else {
                        Vec::new()
                    };

                if !imported.is_empty() {
                    let count = imported.len();
                    for p in imported {
                        if let Some(existing) =
                            self.user_presets.iter_mut().find(|e| e.title == p.title)
                        {
                            *existing = p;
                        } else {
                            self.user_presets.push(p);
                        }
                    }
                    save_presets_json(&self.user_presets);
                    self.status_msg = match self.lang {
                        Lang::Ru => format!("Импортировано пресетов: {}", count),
                        Lang::En => format!("Imported presets: {}", count),
                    };
                }
            }
        }
    }

    fn trigger_update_check(&mut self, ctx: &egui::Context) {
        self.update_state = UpdateCheckState::Checking;
        self.update_rx = Some(updater::start_update_check(
            APP_VERSION,
            GITHUB_REPO_URL,
            ctx.clone(),
        ));
    }

    fn register_explorer_context_menu(&mut self, install: bool) {
        #[cfg(not(windows))]
        let _ = install;
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
            let content = build_rollback_bat(last_batch);
            if fs::write(&save_path, content).is_ok() {
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

    fn select_for_preview(&mut self, path: PathBuf, ctx: &egui::Context) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.selected_path = Some(path.clone());
        self.video_thumb_rx = None;

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

        if photo_meta::is_previewable_image_ext(&ext) {
            let meta = self
                .all_entries
                .iter()
                .find(|e| e.path == path)
                .and_then(|e| e.photo_meta.clone())
                .unwrap_or_else(|| photo_meta::extract_photo_meta(&path, &ext));

            match image::open(&path) {
                Ok(img) => {
                    let mut final_meta = meta;
                    if final_meta.width.is_none() || final_meta.height.is_none() {
                        final_meta.width = Some(img.width());
                        final_meta.height = Some(img.height());
                    }
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
                        meta: final_meta,
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

        if photo_meta::is_photo_meta_ext(&ext) {
            let meta = self
                .all_entries
                .iter()
                .find(|e| e.path == path)
                .and_then(|e| e.photo_meta.clone())
                .unwrap_or_else(|| photo_meta::extract_photo_meta(&path, &ext));
            self.preview_state = PreviewState::PhotoMetaOnly {
                name,
                ext_upper: ext.to_uppercase(),
                meta,
            };
            return;
        }

        if media_info::is_video_ext(&ext) {
            let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let info = self
                .all_entries
                .iter()
                .find(|e| e.path == path)
                .and_then(|e| e.video_info.clone())
                .or_else(|| {
                    media_info::extract_video_info(&path, size_bytes, self.use_external_mediainfo)
                })
                .unwrap_or_default();

            let loading_thumb = media_info::has_ffmpeg();
            if loading_thumb {
                let (tx, rx) = mpsc::channel();
                let p_clone = path.clone();
                let dur = info.duration_secs;
                let ctx_clone = ctx.clone();
                std::thread::spawn(move || {
                    let png = media_info::extract_video_thumbnail_png(&p_clone, dur);
                    let _ = tx.send((p_clone, png));
                    ctx_clone.request_repaint();
                });
                self.video_thumb_rx = Some(rx);
            }

            self.preview_state = PreviewState::Video {
                name,
                info,
                size_bytes,
                thumb: None,
                loading_thumb,
            };
            return;
        }

        if audio_info::is_audio_ext(&ext) {
            let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let info = self
                .all_entries
                .iter()
                .find(|e| e.path == path)
                .and_then(|e| e.audio_info.clone())
                .or_else(|| audio_info::extract_audio_info(&path, size_bytes))
                .unwrap_or_default();
            let cover = audio_info::extract_audio_cover(&path)
                .and_then(|bytes| texture_from_memory(ctx, "item_preview_cover", &bytes, 240));
            self.preview_state = PreviewState::Audio {
                name,
                info,
                size_bytes,
                cover,
            };
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

    fn delete_paths_to_trash(&mut self, targets: Vec<PathBuf>, ctx: &egui::Context) {
        if targets.is_empty() {
            return;
        }
        let count = targets.len();
        match trash::delete_all(&targets) {
            Ok(()) => {
                for p in &targets {
                    self.excluded_paths.remove(p);
                    self.manual_overrides.remove(p);
                    self.hash_cache.remove(p);
                    if self.selected_path.as_ref() == Some(p) {
                        self.selected_path = None;
                        self.preview_state = PreviewState::Empty;
                    }
                }
                if let Some(dir) = self.current_dir.clone() {
                    self.load_directory(dir, ctx);
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

    fn start_duplicate_scan(&mut self, ctx: &egui::Context) {
        if let Some(c) = &self.dup_cancel {
            c.store(true, Ordering::Relaxed);
        }
        let candidates: Vec<(PathBuf, u64)> = self
            .all_entries
            .iter()
            .filter(|e| !e.is_dir && e.size > 0)
            .map(|e| (e.path.clone(), e.size))
            .collect();

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_worker = Arc::clone(&cancel);
        let ctx_clone = ctx.clone();

        self.dup_scanning = true;
        self.dup_progress = (0, candidates.len());
        self.dup_cancel = Some(cancel);
        self.dup_rx = Some(rx);
        self.dup_groups.clear();
        self.dup_checked.clear();

        std::thread::spawn(move || {
            let groups = hash_utils::find_duplicates_bg(candidates, cancel_worker, |done, total| {
                if done == 1 || done == total || done % 4 == 0 {
                    let _ = tx.send(DupScanMsg::Progress { done, total });
                    ctx_clone.request_repaint();
                }
            });
            let _ = tx.send(DupScanMsg::Done(groups));
            ctx_clone.request_repaint();
        });
    }

    fn open_new_directory(&mut self, root_dir: PathBuf, ctx: &egui::Context) {
        if self.current_dir.as_ref() != Some(&root_dir) {
            self.excluded_paths.clear();
            self.manual_overrides.clear();
            self.hash_cache.clear();
            self.selected_path = None;
            self.preview_state = PreviewState::Empty;
            self.editing_row_path = None;
            self.dup_groups.clear();
            self.dup_checked.clear();
        }
        self.load_directory(root_dir, ctx);
    }

    fn load_directory(&mut self, root_dir: PathBuf, ctx: &egui::Context) {
        if let Some(prev) = &self.scan_job {
            prev.cancel.store(true, Ordering::Relaxed);
        }
        self.current_dir = Some(root_dir.clone());
        self.scan_job = Some(scanner::start_background_scan(
            root_dir,
            self.recursive,
            self.hide_system_hidden,
            self.use_external_mediainfo,
            ctx.clone(),
        ));
    }

    fn poll_background_tasks(&mut self, ctx: &egui::Context) {
        let mut finished_scan: Option<ScanMsg> = None;
        if let Some(job) = &mut self.scan_job {
            while let Ok(msg) = job.rx.try_recv() {
                match msg {
                    ScanMsg::Progress {
                        scanned,
                        current_name,
                    } => {
                        job.scanned = scanned;
                        job.current_name = current_name;
                    }
                    done @ ScanMsg::Done { .. } => {
                        finished_scan = Some(done);
                    }
                }
            }
        }
        if let Some(ScanMsg::Done {
            root_dir,
            entries,
            subfolders,
            cancelled,
        }) = finished_scan
        {
            self.scan_job = None;
            if !cancelled {
                self.current_dir = Some(root_dir);
                self.subfolders = subfolders;
                self.all_entries = entries;
                self.recalculate_preview();
            } else {
                self.status_msg = self
                    .lang
                    .tr("Сканирование отменено пользователем.", "Scan cancelled.")
                    .to_string();
            }
        }

        if let Some(rx) = &self.video_thumb_rx {
            if let Ok((target_path, png_opt)) = rx.try_recv() {
                self.video_thumb_rx = None;
                if self.selected_path.as_ref() == Some(&target_path) {
                    if let PreviewState::Video {
                        thumb,
                        loading_thumb,
                        ..
                    } = &mut self.preview_state
                    {
                        *loading_thumb = false;
                        if let Some(bytes) = png_opt {
                            *thumb =
                                texture_from_memory(ctx, "item_preview_video_frame", &bytes, 320);
                        }
                    }
                }
            }
        }

        let mut dup_done: Option<Vec<DuplicateGroup>> = None;
        if let Some(rx) = &self.dup_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    DupScanMsg::Progress { done, total } => {
                        self.dup_progress = (done, total);
                    }
                    DupScanMsg::Done(groups) => {
                        dup_done = Some(groups);
                    }
                }
            }
        }
        if let Some(groups) = dup_done {
            self.dup_scanning = false;
            self.dup_rx = None;
            self.dup_cancel = None;
            self.dup_groups = groups;
        }

        if let Some(rx) = &self.update_rx {
            if let Ok(res) = rx.try_recv() {
                self.update_rx = None;
                self.update_state = match res {
                    Ok(Some(info)) => UpdateCheckState::Available(info),
                    Ok(None) => UpdateCheckState::UpToDate,
                    Err(_) => UpdateCheckState::Failed,
                };
            }
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

    fn recalculate_preview(&mut self) {
        let cfg = self.to_rename_config();
        let summary = compute_preview(
            self.current_dir.as_deref(),
            &self.all_entries,
            &self.excluded_paths,
            &self.manual_overrides,
            &cfg,
            &mut self.hash_cache,
        );
        self.preview_rows = summary.rows;
        self.issue_count = summary.issue_count;

        self.status_msg = match self.lang {
            Lang::Ru => format!(
                "Показано: {} (из {})  •  Выбрано: {}  •  К переименованию: {}{}",
                self.preview_rows.len(),
                self.all_entries.len(),
                summary.included_count,
                summary.changed_count,
                if summary.issue_count > 0 {
                    format!("  •  ⚠ Конфликты: {}", summary.issue_count)
                } else {
                    String::new()
                }
            ),
            Lang::En => format!(
                "Shown: {} (of {})  •  Selected: {}  •  Ready to rename: {}{}",
                self.preview_rows.len(),
                self.all_entries.len(),
                summary.included_count,
                summary.changed_count,
                if summary.issue_count > 0 {
                    format!("  •  ⚠ Issues: {}", summary.issue_count)
                } else {
                    String::new()
                }
            ),
        };
    }

    fn reset_rules(&mut self) {
        self.find_str.clear();
        self.replace_str.clear();
        self.use_regex = false;
        self.case_sensitive = true;
        self.extra_replaces.clear();
        self.rule_order = default_rule_order();
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
        self.auto_suffix_duplicates = false;
        self.sync_sidecars = false;
        self.organize_enabled = false;
        self.organize_template.clear();
        self.manual_overrides.clear();
        self.editing_row_path = None;
        self.recalculate_preview();
    }

    fn apply_rename(&mut self, ctx: &egui::Context) {
        match execute_rename_batch(&self.preview_rows, self.lang) {
            Ok(batch) => {
                let count = batch.moves.len();
                if count > 0 {
                    if self.undo_stack.len() >= 20 {
                        self.undo_stack.remove(0);
                    }
                    self.undo_stack.push(batch);
                }
                self.manual_overrides.clear();
                self.editing_row_path = None;
                if let Some(dir) = self.current_dir.clone() {
                    self.load_directory(dir, ctx);
                }
                self.status_msg = match self.lang {
                    Lang::Ru => format!("Успешно переименовано объектов: {}", count),
                    Lang::En => format!("Successfully renamed items: {}", count),
                };
            }
            Err(err) => {
                self.status_msg = err;
            }
        }
    }

    fn undo_rename(&mut self, ctx: &egui::Context) {
        let Some(last_batch) = self.undo_stack.pop() else {
            return;
        };
        let (restored, err) = execute_undo_batch(last_batch);
        if let Some(dir) = self.current_dir.clone() {
            self.load_directory(dir, ctx);
        }
        self.status_msg = if let Some(e) = err {
            format!("Undo partial ({}): {}", restored, e)
        } else {
            match self.lang {
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
            }
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
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Enter) && !wants_kb {
                trig_apply = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.open_new_directory(folder, ctx);
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
                    self.open_new_directory(dir, ctx);
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

fn render_photo_meta_lines(ui: &mut egui::Ui, meta: &photo_meta::PhotoMeta, lang: Lang) {
    if let Some((w, h)) = meta.dimensions() {
        ui.label(format!("{} × {} px", w, h));
    }
    let mut line1 = Vec::new();
    if let Some(d) = &meta.exif_date {
        let dt = if let Some(t) = &meta.exif_time {
            format!("{} {}", d, t)
        } else {
            d.clone()
        };
        line1.push(format!("📅 {}", dt));
    }
    if let Some(c) = &meta.camera {
        line1.push(format!("📷 {}", c));
    }
    if !line1.is_empty() {
        ui.weak(line1.join("  •  "));
    }
    let mut line2 = Vec::new();
    if let Some(l) = &meta.lens {
        line2.push(format!("🔭 {}", l));
    }
    if let Some(iso) = meta.iso_tag() {
        line2.push(iso);
    }
    if let Some(ap) = meta.aperture_tag() {
        line2.push(ap);
    }
    if let Some(sh) = meta.shutter_display() {
        line2.push(sh);
    }
    if !line2.is_empty() {
        ui.weak(line2.join("  •  "));
    }
    if !meta.has_exif() && meta.dimensions().is_none() {
        ui.weak(lang.tr("EXIF-теги не найдены.", "No EXIF metadata found."));
    }
}

impl eframe::App for ReStemApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_background_tasks(ctx);
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
        let mut trigger_update_check_now = false;
        let mut trigger_dup_scan = false;

        let checked_count = self.preview_rows.iter().filter(|r| r.included).count();
        let ready_to_rename_count = self
            .preview_rows
            .iter()
            .filter(|r| r.included && r.changed)
            .count();
        let can_apply_now = ready_to_rename_count > 0 && self.issue_count == 0;

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
                            lang.tr(
                                "Убрать номер копии в скобках: « (1)», « (2)»",
                                "Remove copy numbers: ' (1)', ' (2)'",
                            ),
                            r"\s*\(\d+\)",
                            "",
                        ),
                        (
                            lang.tr(
                                "Убрать текст в квадратных скобках [тег]",
                                "Remove bracketed tags [tag]",
                            ),
                            r"\s*\[[^\]]*\]",
                            "",
                        ),
                        (
                            lang.tr(
                                "Переставить дату ДД.ММ.ГГГГ → ГГГГ-ММ-ДД",
                                "Swap date DD.MM.YYYY → YYYY-MM-DD",
                            ),
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
                            lang.tr(
                                "Поменять местами 2 слова через дефис (А-Б → Б-А)",
                                "Swap two hyphenated words (A-B → B-A)",
                            ),
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

        // --- МОДАЛЬНОЕ ОКНО: ПОИСК ДУБЛИКАТОВ ---
        if self.show_duplicates_modal {
            let mut open = true;
            let mut trash_dups_now = false;

            egui::Window::new(lang.tr("🧬 Поиск дубликатов файлов", "🧬 Duplicate File Finder"))
                .collapsible(false)
                .resizable(true)
                .default_width(600.0)
                .default_height(430.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if !self.dup_scanning {
                            if ui
                                .add_enabled(
                                    !self.all_entries.is_empty(),
                                    egui::Button::new(lang.tr(
                                        "🔍 Сканировать дубликаты (Размер + SHA-256)",
                                        "🔍 Scan for Duplicates (Size + SHA-256)",
                                    )),
                                )
                                .clicked()
                            {
                                trigger_dup_scan = true;
                            }
                        } else if ui.button(lang.tr("✖ Отмена", "✖ Cancel")).clicked() {
                            if let Some(c) = &self.dup_cancel {
                                c.store(true, Ordering::Relaxed);
                            }
                        }

                        if !self.dup_groups.is_empty() {
                            if ui
                                .button(lang.tr(
                                    "☑ Отметить копии (оставить по 1)",
                                    "☑ Select copies (keep 1st)",
                                ))
                                .clicked()
                            {
                                self.dup_checked.clear();
                                for g in &self.dup_groups {
                                    for p in g.paths.iter().skip(1) {
                                        self.dup_checked.insert(p.clone());
                                    }
                                }
                            }
                            if ui.button(lang.tr("🧹 Снять отметки", "🧹 Clear selection")).clicked()
                            {
                                self.dup_checked.clear();
                            }
                        }
                    });

                    if self.dup_scanning {
                        let (done, total) = self.dup_progress;
                        let frac = if total > 0 {
                            done as f32 / total as f32
                        } else {
                            0.0
                        };
                        ui.add(
                            egui::ProgressBar::new(frac).text(format!(
                                "{} {}/{}",
                                lang.tr("Проверка файлов:", "Checking files:"),
                                done,
                                total
                            )),
                        );
                    }

                    ui.separator();

                    if self.dup_groups.is_empty() && !self.dup_scanning {
                        ui.weak(lang.tr(
                            "Дубликаты не найдены или сканирование ещё не запускалось.",
                            "No duplicates found yet, or scan has not been run.",
                        ));
                    } else if !self.dup_groups.is_empty() {
                        let total_waste: u64 = self
                            .dup_groups
                            .iter()
                            .map(|g| g.file_size.saturating_mul((g.paths.len() - 1) as u64))
                            .sum();
                        ui.strong(match lang {
                            Lang::Ru => format!(
                                "Найдено групп: {}  •  Лишний объём: {}",
                                self.dup_groups.len(),
                                Self::format_size(total_waste, false)
                            ),
                            Lang::En => format!(
                                "Duplicate groups: {}  •  Wasted space: {}",
                                self.dup_groups.len(),
                                Self::format_size(total_waste, false)
                            ),
                        });

                        egui::ScrollArea::vertical()
                            .id_salt("dup_groups_scroll")
                            .max_height(260.0)
                            .show(ui, |ui| {
                                for (idx, g) in self.dup_groups.iter().enumerate() {
                                    card_frame(is_dark).show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.strong(format!(
                                            "#{} • {} • {} {} • SHA256:{}",
                                            idx + 1,
                                            Self::format_size(g.file_size, false),
                                            g.paths.len(),
                                            lang.tr("файла(ов)", "files"),
                                            &g.sha256[..8.min(g.sha256.len())]
                                        ));
                                        for p in &g.paths {
                                            let mut is_chk = self.dup_checked.contains(p);
                                            let disp = if let Some(root) = &self.current_dir {
                                                p.strip_prefix(root)
                                                    .map(|rp| rp.display().to_string())
                                                    .unwrap_or_else(|_| p.display().to_string())
                                            } else {
                                                p.display().to_string()
                                            };
                                            if ui.checkbox(&mut is_chk, disp).changed() {
                                                if is_chk {
                                                    self.dup_checked.insert(p.clone());
                                                } else {
                                                    self.dup_checked.remove(p);
                                                }
                                            }
                                        }
                                    });
                                    ui.add_space(4.0);
                                }
                            });

                        ui.add_space(6.0);
                        let del_cnt = self.dup_checked.len();
                        let del_lbl = match lang {
                            Lang::Ru => format!("🗑 В корзину выбранные копии ({})", del_cnt),
                            Lang::En => format!("🗑 Move selected copies to Trash ({})", del_cnt),
                        };
                        let btn = egui::Button::new(
                            egui::RichText::new(del_lbl).color(egui::Color32::WHITE),
                        )
                        .fill(if del_cnt > 0 {
                            egui::Color32::from_rgb(205, 55, 55)
                        } else {
                            egui::Color32::from_gray(95)
                        });
                        if ui.add_enabled(del_cnt > 0, btn).clicked() {
                            trash_dups_now = true;
                        }
                    }
                });

            if trash_dups_now {
                let targets: Vec<PathBuf> = self.dup_checked.iter().cloned().collect();
                for p in &targets {
                    self.dup_checked.remove(p);
                    for g in &mut self.dup_groups {
                        g.paths.retain(|gp| gp != p);
                    }
                }
                self.dup_groups.retain(|g| g.paths.len() >= 2);
                self.delete_paths_to_trash(targets, ctx);
            }
            if !open {
                self.show_duplicates_modal = false;
            }
        }

        // --- МОДАЛЬНОЕ ОКНО «О ПРОГРАММЕ» ---
        if self.show_about_modal {
            let mut open = true;
            let mut close_about = false;
            let mut reg_action: Option<bool> = None;

            egui::Window::new(lang.tr("ℹ О программе ReStem", "ℹ About ReStem"))
                .collapsible(false)
                .resizable(false)
                .default_width(450.0)
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
                                    "{}: v{} • {}",
                                    lang.tr("Версия", "Version"),
                                    APP_VERSION,
                                    APP_LICENSE
                                ),
                            );
                        });
                    });

                    ui.add_space(8.0);
                    card_frame(is_dark).show(ui, |ui| {
                        ui.set_min_width(410.0);
                        ui.label(lang.tr(
                            "• Рекурсивный асинхронный скан + раскладка в подпапки по шаблону\n\
                             • Фото EXIF (JPEG/HEIC/WebP/PNG/RAW): {exif_date}, {exif_time}, {camera}, {lens}, {iso}, {aperture}, {shutter}\n\
                             • Видео MediaInfo: {res}, {vcodec}, {acodec}, {ach}, {fps}, {duration}, {bitrate}, {hdr}, {media_date}\n\
                             • Аудио-теги (ID3/FLAC/OGG/M4A/WAV): {artist}, {album}, {title}, {track}, {audio_year}, {genre}\n\
                             • Хеши {crc32}, {sha256_8}, {size}; парные файлы (.srt/.xmp); поиск дубликатов\n\
                             • Настройка порядка правил (Pipeline), ручная правка строк, пресеты JSON и CLI-режим",
                            "• Async recursive scan & template subfolder organization\n\
                             • Photo EXIF (JPEG/HEIC/WebP/PNG/RAW): {exif_date}, {exif_time}, {camera}, {lens}, {iso}, {aperture}, {shutter}\n\
                             • Video MediaInfo: {res}, {vcodec}, {acodec}, {ach}, {fps}, {duration}, {bitrate}, {hdr}, {media_date}\n\
                             • Audio tags (ID3/FLAC/OGG/M4A/WAV): {artist}, {album}, {title}, {track}, {audio_year}, {genre}\n\
                             • Hashes {crc32}, {sha256_8}, {size}; sidecar sync (.srt/.xmp); duplicate finder\n\
                             • Custom rule pipeline order, inline row edit, JSON presets & CLI automation",
                        ));

                        ui.separator();
                        ui.strong(lang.tr(
                            "Официальный репозиторий GitHub и обновления:",
                            "Official GitHub Repository & Updates:",
                        ));
                        ui.weak(GITHUB_REPO_URL);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .button(lang.tr("🌐 Открыть GitHub", "🌐 Open GitHub Repo"))
                                .clicked()
                            {
                                open_url_in_browser(GITHUB_REPO_URL);
                            }
                            if ui
                                .button(lang.tr("🔄 Проверить обновления", "🔄 Check Updates"))
                                .clicked()
                            {
                                trigger_update_check_now = true;
                            }
                            if ui
                                .button(lang.tr("⬇ Страница Releases", "⬇ Releases Page"))
                                .clicked()
                            {
                                let rel_url = format!(
                                    "{}/releases/latest",
                                    GITHUB_REPO_URL.trim_end_matches('/')
                                );
                                open_url_in_browser(&rel_url);
                            }
                        });

                        if ui
                            .checkbox(
                                &mut self.check_updates,
                                lang.tr(
                                    "Проверять обновления при запуске программы",
                                    "Check for updates on startup",
                                ),
                            )
                            .changed()
                        {
                            settings_dirty = true;
                        }

                        match &self.update_state {
                            UpdateCheckState::Checking => {
                                ui.weak(lang.tr("Проверка обновлений...", "Checking for updates..."));
                            }
                            UpdateCheckState::UpToDate => {
                                ui.colored_label(
                                    egui::Color32::from_rgb(24, 150, 105),
                                    lang.tr("✔ Установлена актуальная версия.", "✔ You are up to date."),
                                );
                            }
                            UpdateCheckState::Available(info) => {
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(35, 190, 115),
                                        format!(
                                            "{} v{}",
                                            lang.tr("Доступна новая версия:", "New version available:"),
                                            info.latest_version
                                        ),
                                    );
                                    if ui.small_button(lang.tr("Скачать", "Download")).clicked() {
                                        open_url_in_browser(&info.release_url);
                                    }
                                });
                            }
                            UpdateCheckState::Failed => {
                                ui.weak(lang.tr(
                                    "Не удалось проверить обновления.",
                                    "Failed to check for updates.",
                                ));
                            }
                            UpdateCheckState::Idle => {}
                        }

                        ui.separator();
                        ui.strong(lang.tr(
                            "Интеграция в контекстное меню Проводника Windows:",
                            "Windows Explorer Context Menu Integration:",
                        ));
                        ui.horizontal_wrapped(|ui| {
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
                        ui.weak(lang.tr(
                            "CLI: restem <ПАПКА> --preset <ИМЯ> [--dry-run | --apply]",
                            "CLI: restem <DIR> --preset <NAME> [--dry-run | --apply]",
                        ));
                        ui.horizontal_wrapped(|ui| {
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
                                "ReStem v{} ({}) — {}",
                                APP_VERSION, APP_LICENSE, GITHUB_REPO_URL
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
                    ui.horizontal_wrapped(|ui| {
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
                self.delete_paths_to_trash(targets, ctx);
            } else if close_now || !open {
                self.confirm_trash_modal = false;
            }
        }

        // --- ВЕРХНЯЯ ПАНЕЛЬ ИНСТРУМЕНТОВ ---
        egui::TopBottomPanel::top("top_toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
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

                if let UpdateCheckState::Available(info) = &self.update_state {
                    let upd_btn = egui::Button::new(
                        egui::RichText::new(format!("⬆ v{}", info.latest_version))
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(egui::Color32::from_rgb(16, 150, 92));
                    if ui
                        .add(upd_btn)
                        .on_hover_text(lang.tr(
                            "Доступна новая версия на GitHub — нажмите для скачивания",
                            "New release available on GitHub — click to download",
                        ))
                        .clicked()
                    {
                        open_url_in_browser(&info.release_url);
                    }
                }

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

                if ui
                    .button(lang.tr("🧬 Дубликаты", "🧬 Duplicates"))
                    .on_hover_text(lang.tr(
                        "Поиск точных копий файлов в папке",
                        "Find exact duplicate files in folder",
                    ))
                    .clicked()
                {
                    self.show_duplicates_modal = true;
                }

                let top_apply_label = if self.issue_count > 0 {
                    match lang {
                        Lang::Ru => format!("⚠ Конфликты ({})", self.issue_count),
                        Lang::En => format!("⚠ Issues ({})", self.issue_count),
                    }
                } else {
                    match lang {
                        Lang::Ru => format!("✔ Применить ({})", ready_to_rename_count),
                        Lang::En => format!("✔ Apply ({})", ready_to_rename_count),
                    }
                };
                let top_apply_fill = if self.issue_count > 0 {
                    egui::Color32::from_rgb(190, 60, 50)
                } else if ready_to_rename_count > 0 {
                    egui::Color32::from_rgb(16, 148, 92)
                } else if is_dark {
                    egui::Color32::from_rgb(42, 48, 60)
                } else {
                    egui::Color32::from_rgb(210, 216, 226)
                };
                let top_apply_fg = if can_apply_now || self.issue_count > 0 {
                    egui::Color32::WHITE
                } else if is_dark {
                    egui::Color32::from_gray(160)
                } else {
                    egui::Color32::from_gray(105)
                };
                let top_apply_btn = egui::Button::new(
                    egui::RichText::new(top_apply_label)
                        .color(top_apply_fg)
                        .strong(),
                )
                .fill(top_apply_fill);

                if ui
                    .add_enabled(can_apply_now, top_apply_btn)
                    .on_hover_text("Ctrl+Enter")
                    .clicked()
                {
                    trigger_apply = true;
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
                        "Экспортировать .bat-скрипт для отката",
                        "Export .bat rollback script",
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

                if ui
                    .small_button("A-")
                    .on_hover_text(lang.tr("Уменьшить масштаб", "Zoom Out"))
                    .clicked()
                {
                    self.ui_scale = (self.ui_scale - 0.1).max(0.85);
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
                    .small_button("A+")
                    .on_hover_text(lang.tr("Увеличить масштаб", "Zoom In"))
                    .clicked()
                {
                    self.ui_scale = (self.ui_scale + 0.1).min(1.5);
                    ctx.set_pixels_per_point(self.ui_scale);
                    settings_dirty = true;
                }

                if ui.selectable_label(self.lang == Lang::Ru, "RU").clicked() {
                    self.lang = Lang::Ru;
                    settings_dirty = true;
                    needs_recalc = true;
                }
                if ui.selectable_label(self.lang == Lang::En, "EN").clicked() {
                    self.lang = Lang::En;
                    settings_dirty = true;
                    needs_recalc = true;
                }

                if ui
                    .button("🌐 GitHub")
                    .on_hover_text(GITHUB_REPO_URL)
                    .clicked()
                {
                    open_url_in_browser(GITHUB_REPO_URL);
                }
                if ui
                    .button(lang.tr("ℹ О программе", "ℹ About"))
                    .clicked()
                {
                    self.show_about_modal = true;
                }
            });

            ui.separator();

            // Ряд 2: Поиск, фильтрация, Diff-режим и сортировка
            ui.horizontal_wrapped(|ui| {
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
                    .on_hover_text(lang.tr("Учитывать регистр", "Case-sensitive"))
                    .clicked()
                {
                    self.search_case_sensitive = !self.search_case_sensitive;
                    needs_recalc = true;
                }
                if ui
                    .selectable_label(self.search_regex, ".*")
                    .on_hover_text("Regex")
                    .clicked()
                {
                    self.search_regex = !self.search_regex;
                    needs_recalc = true;
                }
                if !self.search_query.is_empty() && ui.small_button("✖").clicked() {
                    self.search_query.clear();
                    needs_recalc = true;
                }

                ui.separator();

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
                        lang.tr("🎨 Diff-буквы", "🎨 Inline Diff"),
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
                        StatusFilter::OnlyIssues => lang.tr("⚠ Конфликты / Ошибки", "⚠ Issues only"),
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
                        needs_recalc |= ui
                            .selectable_value(
                                &mut self.status_filter,
                                StatusFilter::OnlyIssues,
                                lang.tr("⚠ Конфликты / Ошибки", "⚠ Issues only"),
                            )
                            .changed();
                    });

                ui.separator();

                ui.strong(lang.tr("Сортировка:", "Sort:"));
                egui::ComboBox::from_id_salt("sort_field_cb")
                    .width(165.0)
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

        // --- НИЖНЯЯ СТРОКА СОСТОЯНИЯ + ИНДИКАТОР СКАНИРОВАНИЯ ---
        egui::TopBottomPanel::bottom("status_footer").show(ctx, |ui| {
            ui.add_space(2.0);
            if let Some(job) = &self.scan_job {
                ui.horizontal_wrapped(|ui| {
                    ui.spinner();
                    ui.strong(format!(
                        "{}: {} ({})",
                        lang.tr("Сканирование", "Scanning"),
                        job.scanned,
                        job.current_name
                    ));
                    if ui.small_button(lang.tr("✖ Отмена", "✖ Cancel")).clicked() {
                        job.cancel.store(true, Ordering::Relaxed);
                    }
                });
            } else {
                let path_str = self
                    .current_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| {
                        lang.tr("Папка не выбрана", "No folder selected").to_string()
                    });

                if ui.available_width() >= 900.0 {
                    ui.horizontal(|ui| {
                        ui.strong(&self.status_msg);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(path_str).truncate());
                        });
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(&self.status_msg);
                        ui.separator();
                        ui.weak(path_str);
                    });
                }
            }
            ui.add_space(2.0);
        });

        let screen_w = ctx.screen_rect().width();
        let left_max_w = (screen_w * 0.27).clamp(215.0, 390.0);
        let right_max_w = (screen_w * 0.34).clamp(270.0, 460.0);

        // --- ЛЕВАЯ КОЛОНКА ---
        egui::SidePanel::left("left_explorer_panel")
            .resizable(true)
            .default_width(275.0)
            .width_range(200.0..=left_max_w)
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
                                .max_height(130.0)
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
                                    meta,
                                    tex_size,
                                } => {
                                    ui.strong(format!("🖼 {}", name));
                                    render_photo_meta_lines(ui, meta, lang);
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
                                PreviewState::PhotoMetaOnly {
                                    name,
                                    ext_upper,
                                    meta,
                                } => {
                                    ui.strong(format!("📷 {}", name));
                                    ui.weak(format!("{} (EXIF)", ext_upper));
                                    render_photo_meta_lines(ui, meta, lang);
                                }
                                PreviewState::Video {
                                    name,
                                    info,
                                    size_bytes,
                                    thumb,
                                    loading_thumb,
                                } => {
                                    ui.strong(format!("🎬 {}", name));
                                    if let Some((tex, tex_size)) = thumb {
                                        let max_w = ui.available_width().max(80.0);
                                        let max_h = 155.0;
                                        let scale = (max_w / tex_size.x)
                                            .min(max_h / tex_size.y)
                                            .min(1.0);
                                        let draw_size = *tex_size * scale;
                                        let (rect, _) =
                                            ui.allocate_exact_size(draw_size, egui::Sense::hover());
                                        ui.painter().image(
                                            tex.id(),
                                            rect,
                                            egui::Rect::from_min_max(
                                                egui::pos2(0.0, 0.0),
                                                egui::pos2(1.0, 1.0),
                                            ),
                                            egui::Color32::WHITE,
                                        );
                                    } else if *loading_thumb {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.weak(lang.tr(
                                                "Загрузка кадра (ffmpeg)...",
                                                "Extracting video frame (ffmpeg)...",
                                            ));
                                        });
                                    }
                                    if let Some((w, h)) = info.dimensions() {
                                        let res_lbl = info
                                            .resolution_label()
                                            .map(|r| format!(" ({})", r))
                                            .unwrap_or_default();
                                        let hdr_lbl = info
                                            .hdr
                                            .as_ref()
                                            .map(|h| format!(" • {}", h))
                                            .unwrap_or_default();
                                        ui.label(format!("{} × {} px{}{}", w, h, res_lbl, hdr_lbl));
                                    }
                                    let mut video_parts = Vec::new();
                                    if let Some(vc) = &info.video_codec {
                                        video_parts.push(vc.clone());
                                    }
                                    if let Some(fps) = info.fps_tag() {
                                        video_parts.push(fps);
                                    }
                                    if !video_parts.is_empty() {
                                        ui.label(format!(
                                            "{}: {}",
                                            lang.tr("Видео", "Video"),
                                            video_parts.join(" • ")
                                        ));
                                    }
                                    let mut audio_parts = Vec::new();
                                    if let Some(ac) = &info.audio_codec {
                                        audio_parts.push(ac.clone());
                                    }
                                    if let Some(ch) = info.audio_channels_tag() {
                                        audio_parts.push(ch);
                                    }
                                    if !audio_parts.is_empty() {
                                        ui.label(format!(
                                            "{}: {}",
                                            lang.tr("Аудио", "Audio"),
                                            audio_parts.join(" • ")
                                        ));
                                    }
                                    let mut meta_parts = Vec::new();
                                    if let Some(dur) = info.duration_display() {
                                        meta_parts.push(format!("⏱ {}", dur));
                                    }
                                    if let Some(br) = info.bitrate_tag() {
                                        meta_parts.push(br);
                                    } else if *size_bytes > 0 {
                                        meta_parts.push(Self::format_size(*size_bytes, false));
                                    }
                                    if !meta_parts.is_empty() {
                                        ui.label(meta_parts.join("  •  "));
                                    }
                                    if let Some(rd) = &info.recorded_date {
                                        ui.weak(format!(
                                            "📅 {}: {}",
                                            lang.tr("Дата записи", "Recorded"),
                                            rd
                                        ));
                                    }
                                    let cont = info.container.as_deref().unwrap_or("Video");
                                    ui.weak(format!(
                                        "{}: {} ({})",
                                        lang.tr("Контейнер", "Container"),
                                        cont,
                                        info.source.label()
                                    ));
                                    if !info.has_any_metadata() {
                                        ui.weak(lang.tr(
                                            "Метаданные не найдены. Для нестандартных форматов установите MediaInfo CLI или ffprobe в PATH.",
                                            "No metadata found. For uncommon formats, install MediaInfo CLI or ffprobe in PATH.",
                                        ));
                                    }
                                }
                                PreviewState::Audio {
                                    name,
                                    info,
                                    size_bytes,
                                    cover,
                                } => {
                                    ui.strong(format!("🎵 {}", name));
                                    if let Some((tex, tex_size)) = cover {
                                        let max_w = ui.available_width().max(80.0);
                                        let max_h = 150.0;
                                        let scale = (max_w / tex_size.x)
                                            .min(max_h / tex_size.y)
                                            .min(1.0);
                                        let draw_size = *tex_size * scale;
                                        let (rect, _) =
                                            ui.allocate_exact_size(draw_size, egui::Sense::hover());
                                        ui.painter().image(
                                            tex.id(),
                                            rect,
                                            egui::Rect::from_min_max(
                                                egui::pos2(0.0, 0.0),
                                                egui::pos2(1.0, 1.0),
                                            ),
                                            egui::Color32::WHITE,
                                        );
                                    }
                                    if info.artist.is_some() || info.title.is_some() {
                                        ui.label(format!(
                                            "{} — {}",
                                            info.artist.as_deref().unwrap_or("?"),
                                            info.title.as_deref().unwrap_or("?")
                                        ));
                                    }
                                    let mut alb_parts = Vec::new();
                                    if let Some(al) = &info.album {
                                        alb_parts.push(al.clone());
                                    }
                                    if let Some(y) = &info.year {
                                        alb_parts.push(y.clone());
                                    }
                                    if let Some(tr) = info.track_tag() {
                                        alb_parts.push(format!("#{}", tr));
                                    }
                                    if !alb_parts.is_empty() {
                                        ui.weak(alb_parts.join("  •  "));
                                    }
                                    if let Some(g) = &info.genre {
                                        ui.weak(format!(
                                            "{}: {}",
                                            lang.tr("Жанр", "Genre"),
                                            g
                                        ));
                                    }
                                    let mut tech = Vec::new();
                                    if let Some(dur) = info.duration_display() {
                                        tech.push(format!("⏱ {}", dur));
                                    }
                                    if let Some(br) = info.bitrate_tag() {
                                        tech.push(br);
                                    }
                                    if let Some(sr) = info.sample_rate_tag() {
                                        tech.push(sr);
                                    }
                                    if let Some(fmt) = &info.format {
                                        tech.push(fmt.clone());
                                    } else if *size_bytes > 0 {
                                        tech.push(Self::format_size(*size_bytes, false));
                                    }
                                    if !tech.is_empty() {
                                        ui.label(tech.join(" • "));
                                    }
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
                                ui.horizontal_wrapped(|ui| {
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
                            if ui
                                .button(lang.tr(
                                    "• Видео: _{res}_{vcodec}_{fps}",
                                    "• Video: _{res}_{vcodec}_{fps}",
                                ))
                                .clicked()
                            {
                                self.suffix = "_{res}_{vcodec}_{fps}".to_string();
                                needs_recalc = true;
                            }
                            if ui
                                .button(lang.tr(
                                    "• Видео: {media_date}_ + _{res}_{acodec}",
                                    "• Video: {media_date}_ + _{res}_{acodec}",
                                ))
                                .clicked()
                            {
                                self.prefix = "{media_date}_".to_string();
                                self.suffix = "_{res}_{acodec}".to_string();
                                needs_recalc = true;
                            }
                            if ui
                                .button(lang.tr(
                                    "• Музыка: {track}_{artist} - {title}",
                                    "• Music: {track}_{artist} - {title}",
                                ))
                                .clicked()
                            {
                                self.find_str = ".*".to_string();
                                self.replace_str = "{track}_{artist} - {title}".to_string();
                                self.use_regex = true;
                                needs_recalc = true;
                            }
                            if ui
                                .button(lang.tr(
                                    "• Фото по дате: {exif_date}/ + {exif_time}",
                                    "• Organize photos: {exif_date}/ + {exif_time}",
                                ))
                                .clicked()
                            {
                                self.prefix = "{exif_date}_{exif_time}_".to_string();
                                self.organize_enabled = true;
                                self.organize_template = "{exif_date}".to_string();
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
                                    let preset = self.user_presets[idx].clone();
                                    self.apply_full_preset(&preset);
                                    needs_recalc = true;
                                }
                                if let Some(idx) = remove_idx {
                                    self.user_presets.remove(idx);
                                    settings_dirty = true;
                                }
                            }

                            ui.separator();
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_preset_title)
                                        .hint_text(lang.tr("Имя пресета...", "Preset name..."))
                                        .desired_width(130.0),
                                );
                                if ui
                                    .button(lang.tr("➕ Сохранить", "➕ Save"))
                                    .clicked()
                                    && !self.new_preset_title.trim().is_empty()
                                {
                                    let title = self.new_preset_title.trim().to_string();
                                    let preset = self.capture_full_preset(title);
                                    self.user_presets.push(preset);
                                    self.new_preset_title.clear();
                                    settings_dirty = true;
                                }
                            });
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .small_button(lang.tr("📤 Экспорт .json", "📤 Export .json"))
                                    .clicked()
                                {
                                    self.export_presets_json_dialog();
                                }
                                if ui
                                    .small_button(lang.tr("📥 Импорт .json", "📥 Import .json"))
                                    .clicked()
                                {
                                    self.import_presets_json_dialog();
                                }
                            });
                        });
                    });
            });

        // --- ПРАВАЯ КОЛОНКА: ПРАВИЛА ---
        egui::SidePanel::right("right_rules_panel")
            .resizable(true)
            .default_width(345.0)
            .width_range(265.0..=right_max_w)
            .show(ctx, |ui| {
                let (dock_bg, dock_border) = if self.issue_count > 0 {
                    if is_dark {
                        (
                            egui::Color32::from_rgb(55, 22, 24),
                            egui::Color32::from_rgb(220, 75, 65),
                        )
                    } else {
                        (
                            egui::Color32::from_rgb(252, 232, 232),
                            egui::Color32::from_rgb(205, 55, 45),
                        )
                    }
                } else if ready_to_rename_count > 0 {
                    if is_dark {
                        (
                            egui::Color32::from_rgb(20, 38, 32),
                            egui::Color32::from_rgb(35, 185, 120),
                        )
                    } else {
                        (
                            egui::Color32::from_rgb(232, 248, 240),
                            egui::Color32::from_rgb(22, 160, 100),
                        )
                    }
                } else if is_dark {
                    (
                        egui::Color32::from_rgb(25, 29, 38),
                        egui::Color32::from_rgb(52, 60, 76),
                    )
                } else {
                    (
                        egui::Color32::from_rgb(236, 240, 246),
                        egui::Color32::from_rgb(205, 212, 224),
                    )
                };

                let sticky_frame = egui::Frame::none()
                    .fill(dock_bg)
                    .stroke(egui::Stroke::new(1.5_f32, dock_border))
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0));

                egui::TopBottomPanel::bottom("right_sticky_apply_bar")
                    .frame(sticky_frame)
                    .show_inside(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if self.issue_count > 0 {
                                ui.colored_label(
                                    egui::Color32::from_rgb(235, 85, 75),
                                    match lang {
                                        Lang::Ru => format!(
                                            "⚠ Конфликты / Ошибки: {} шт.",
                                            self.issue_count
                                        ),
                                        Lang::En => format!(
                                            "⚠ Conflicts / Issues: {}",
                                            self.issue_count
                                        ),
                                    },
                                );
                            } else if ready_to_rename_count > 0 {
                                let badge_col = if is_dark {
                                    egui::Color32::from_rgb(95, 235, 160)
                                } else {
                                    egui::Color32::from_rgb(12, 120, 65)
                                };
                                ui.colored_label(
                                    badge_col,
                                    match lang {
                                        Lang::Ru => format!(
                                            "⚡ К переименованию: {} шт.",
                                            ready_to_rename_count
                                        ),
                                        Lang::En => format!(
                                            "⚡ Ready to rename: {} items",
                                            ready_to_rename_count
                                        ),
                                    },
                                );
                            } else {
                                ui.weak(lang.tr(
                                    "Задайте правила выше",
                                    "Adjust rules above",
                                ));
                            }

                            if ui
                                .small_button(lang.tr("✖ Сброс правил", "✖ Reset Rules"))
                                .clicked()
                            {
                                trigger_reset = true;
                            }
                        });

                        ui.add_space(4.0);

                        let main_btn_label = if self.issue_count > 0 {
                            lang.tr(
                                "⚠ Исправьте конфликты или включите автосуффикс",
                                "⚠ Resolve conflicts or enable auto-suffix",
                            )
                            .to_string()
                        } else if ready_to_rename_count > 0 {
                            match lang {
                                Lang::Ru => {
                                    format!("✔ Применить изменения ({})", ready_to_rename_count)
                                }
                                Lang::En => {
                                    format!("✔ Apply Changes ({})", ready_to_rename_count)
                                }
                            }
                        } else {
                            lang.tr(
                                "✔ Применить изменения (Ctrl+Enter)",
                                "✔ Apply Changes (Ctrl+Enter)",
                            )
                            .to_string()
                        };

                        let btn_fill = if self.issue_count > 0 {
                            egui::Color32::from_rgb(175, 55, 50)
                        } else if ready_to_rename_count > 0 {
                            egui::Color32::from_rgb(16, 150, 92)
                        } else {
                            egui::Color32::from_rgb(18, 118, 215)
                        };

                        let btn_text = egui::RichText::new(main_btn_label)
                            .color(egui::Color32::WHITE)
                            .strong()
                            .size(14.5);

                        let apply_btn = egui::Button::new(btn_text)
                            .fill(btn_fill)
                            .rounding(7.0)
                            .min_size(egui::vec2(ui.available_width(), 42.0));

                        if ui.add_enabled(can_apply_now, apply_btn).clicked() {
                            trigger_apply = true;
                        }
                    });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.heading(lang.tr("Правила", "Rename Rules"));
                    let list_btn_txt = if self.use_custom_list {
                        lang.tr("📋 По списку (ВКЛ)", "📋 From List (ON)")
                    } else {
                        lang.tr("📋 По списку...", "📋 From List...")
                    };
                    if ui.button(list_btn_txt).clicked() {
                        self.show_list_rename_modal = true;
                    }
                });
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("rules_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let combo_w = (ui.available_width() - 24.0).max(160.0);

                        // 1. Поиск и замена + доп. шаги
                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                section_title(
                                    ui,
                                    lang.tr("1. Поиск и замена", "1. Find & Replace"),
                                    is_dark,
                                );
                                if ui.small_button("[?] Regex").clicked() {
                                    self.show_regex_help_modal = true;
                                }
                                if ui
                                    .small_button(lang.tr("➕ Шаг", "➕ Step"))
                                    .on_hover_text(lang.tr(
                                        "Добавить дополнительный шаг замены",
                                        "Add chained find/replace step",
                                    ))
                                    .clicked()
                                {
                                    self.extra_replaces.push(ReplaceStep {
                                        find: String::new(),
                                        replace: String::new(),
                                        use_regex: false,
                                        case_sensitive: true,
                                    });
                                }
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

                            let mut rm_step: Option<usize> = None;
                            for (idx, step) in self.extra_replaces.iter_mut().enumerate() {
                                ui.separator();
                                ui.push_id(format!("extra_rep_{}", idx), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.weak(format!(
                                            "{} #{}",
                                            lang.tr("Шаг", "Step"),
                                            idx + 2
                                        ));
                                        if ui.small_button("✖").clicked() {
                                            rm_step = Some(idx);
                                        }
                                    });
                                    needs_recalc |= ui
                                        .add(
                                            egui::TextEdit::singleline(&mut step.find)
                                                .hint_text(lang.tr("Найти...", "Find...")),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .add(
                                            egui::TextEdit::singleline(&mut step.replace)
                                                .hint_text(lang.tr("Заменить на...", "Replace...")),
                                        )
                                        .changed();
                                    ui.horizontal(|ui| {
                                        needs_recalc |=
                                            ui.checkbox(&mut step.use_regex, "Regex").changed();
                                        needs_recalc |= ui
                                            .checkbox(
                                                &mut step.case_sensitive,
                                                lang.tr("Регистр", "Case"),
                                            )
                                            .changed();
                                    });
                                });
                            }
                            if let Some(idx) = rm_step {
                                self.extra_replaces.remove(idx);
                                needs_recalc = true;
                            }
                        });

                        ui.add_space(8.0);

                        // 2. Регистр, транслит и чистка
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
                                .width(combo_w)
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
                                .width(combo_w)
                                .selected_text(match self.translit_mode {
                                    TranslitMode::None => lang.tr("Выключена", "Disabled"),
                                    TranslitMode::RuToLat => lang.tr(
                                        "Кириллица → Латиница (RU → LAT)",
                                        "Cyrillic → Latin (RU → LAT)",
                                    ),
                                    TranslitMode::LatToRu => lang.tr(
                                        "Латиница → Кириллица (LAT → RU)",
                                        "Latin → Cyrillic (LAT → RU)",
                                    ),
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
                                            lang.tr(
                                                "Кириллица → Латиница (RU → LAT)",
                                                "Cyrillic → Latin (RU → LAT)",
                                            ),
                                        )
                                        .changed();
                                    needs_recalc |= ui
                                        .selectable_value(
                                            &mut self.translit_mode,
                                            TranslitMode::LatToRu,
                                            lang.tr(
                                                "Латиница → Кириллица (LAT → RU)",
                                                "Latin → Cyrillic (LAT → RU)",
                                            ),
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

                        // 3. Вставка, обрезка и Умные теги
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
                                "Фото EXIF, даты, размеры и хеши (в Префикс):",
                                "Photo EXIF, dates, size & hashes (to Prefix):",
                            ));
                            ui.horizontal_wrapped(|ui| {
                                for (tag, tip) in [
                                    (
                                        "{date}_",
                                        lang.tr("Дата изменения файла", "Modified date"),
                                    ),
                                    (
                                        "{exif_date}_",
                                        lang.tr("Дата съёмки фото EXIF", "EXIF capture date"),
                                    ),
                                    (
                                        "{exif_time}_",
                                        lang.tr(
                                            "Время съёмки EXIF (ЧЧ-ММ-СС)",
                                            "EXIF capture time (HH-MM-SS)",
                                        ),
                                    ),
                                    (
                                        "{camera}_",
                                        lang.tr("Модель камеры EXIF", "EXIF camera model"),
                                    ),
                                    (
                                        "{lens}_",
                                        lang.tr("Модель объектива EXIF", "EXIF lens model"),
                                    ),
                                    ("{iso}_", lang.tr("Чувствительность ISO", "EXIF ISO")),
                                    ("{aperture}_", lang.tr("Диафрагма (f2.8)", "Aperture")),
                                    ("{shutter}_", lang.tr("Выдержка (1-500s)", "Shutter speed")),
                                    ("{parent}_", lang.tr("Родительская папка", "Parent folder")),
                                    (
                                        "{w}x{h}_",
                                        lang.tr("Разрешение фото/видео", "Photo/Video dimensions"),
                                    ),
                                    ("{size}_", lang.tr("Размер файла (15MB)", "File size tag")),
                                    ("{crc32}_", lang.tr("Контрольная сумма CRC32", "CRC32 hash")),
                                    (
                                        "{sha256_8}_",
                                        lang.tr("Первые 8 симв. SHA-256", "First 8 chars of SHA-256"),
                                    ),
                                ] {
                                    if ui.small_button(tag).on_hover_text(tip).clicked() {
                                        self.prefix.push_str(tag);
                                        needs_recalc = true;
                                    }
                                }
                            });

                            ui.add_space(4.0);
                            ui.weak(lang.tr(
                                "🎬 Видео-теги MediaInfo (в Суффикс):",
                                "🎬 Video MediaInfo tags (to Suffix):",
                            ));
                            ui.horizontal_wrapped(|ui| {
                                for (tag, tip) in [
                                    ("_{res}", lang.tr("Качество: 1080p, 2160p", "Quality tier")),
                                    ("_{vcodec}", lang.tr("Видеокодек: H264, HEVC, AV1", "Video codec")),
                                    ("_{fps}", lang.tr("Кадров/сек: 24fps, 60fps", "Frame rate")),
                                    ("_{acodec}", lang.tr("Аудиокодек: AAC, AC3, Opus", "Audio codec")),
                                    ("_{ach}", lang.tr("Каналы: 2.0, 5.1, 7.1", "Audio channels")),
                                    ("_{duration}", lang.tr("Длительность (04m35s)", "Duration")),
                                    ("_{bitrate}", lang.tr("Битрейт (12Mbps)", "Bitrate")),
                                    ("_{hdr}", lang.tr("HDR-формат (HDR10, HLG, DV)", "HDR format")),
                                    ("_{media_date}", lang.tr("Дата записи видео", "Video recorded date")),
                                ] {
                                    if ui.small_button(tag).on_hover_text(tip).clicked() {
                                        self.suffix.push_str(tag);
                                        needs_recalc = true;
                                    }
                                }
                            });

                            ui.add_space(4.0);
                            ui.weak(lang.tr(
                                "🎵 Аудио-теги (в Суффикс):",
                                "🎵 Audio tags (to Suffix):",
                            ));
                            ui.horizontal_wrapped(|ui| {
                                for (tag, tip) in [
                                    ("_{track}", lang.tr("Номер трека (01, 02)", "Track number")),
                                    ("_{artist}", lang.tr("Исполнитель", "Artist")),
                                    ("_{title}", lang.tr("Название трека", "Track title")),
                                    ("_{album}", lang.tr("Альбом", "Album")),
                                    ("_{audio_year}", lang.tr("Год релиза", "Release year")),
                                    ("_{genre}", lang.tr("Жанр", "Genre")),
                                    ("_{audio_bitrate}", lang.tr("Битрейт аудио", "Audio bitrate")),
                                    ("_{sample_rate}", lang.tr("Частота (44.1kHz)", "Sample rate")),
                                ] {
                                    if ui.small_button(tag).on_hover_text(tip).clicked() {
                                        self.suffix.push_str(tag);
                                        needs_recalc = true;
                                    }
                                }
                            });
                        });

                        ui.add_space(8.0);

                        // 4. Нумерация
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
                                .width(combo_w)
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

                            ui.horizontal_wrapped(|ui| {
                                ui.label(lang.tr("Старт:", "Start:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_start).range(0..=99999))
                                    .changed();
                                ui.label(lang.tr("Шаг:", "Step:"));
                                needs_recalc |= ui
                                    .add(egui::DragValue::new(&mut self.num_step).range(1..=1000))
                                    .changed();
                            });

                            ui.horizontal_wrapped(|ui| {
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

                        // 5. Расширения файлов
                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr("5. Расширения файлов", "5. File Extensions"),
                                is_dark,
                            );
                            ui.add_space(4.0);
                            egui::ComboBox::from_id_salt("ext_mode_cb")
                                .width(combo_w)
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

                        // 6. Фильтры и исключения
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
                                .width(combo_w)
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
                                ui.horizontal_wrapped(|ui| {
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

                        ui.add_space(8.0);

                        // 7. Безопасность, парные файлы, раскладка и порядок правил
                        card_frame(is_dark).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            section_title(
                                ui,
                                lang.tr(
                                    "7. Безопасность, парные и раскладка",
                                    "7. Safety, Sidecars & Organize",
                                ),
                                is_dark,
                            );
                            ui.add_space(4.0);

                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.auto_suffix_duplicates,
                                    lang.tr(
                                        "🔢 Автосуффикс при дубликатах (_1, _2)",
                                        "🔢 Auto-suffix duplicate names (_1, _2)",
                                    ),
                                )
                                .changed();

                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.sync_sidecars,
                                    lang.tr(
                                        "📎 Парные файлы (.srt, .ass, .vtt, .xmp)",
                                        "📎 Sync sidecar files (.srt, .ass, .vtt, .xmp)",
                                    ),
                                )
                                .changed();

                            needs_recalc |= ui
                                .checkbox(
                                    &mut self.organize_enabled,
                                    lang.tr(
                                        "📂 Раскладка файлов по подпапкам",
                                        "📂 Move files into subfolders (Organize)",
                                    ),
                                )
                                .changed();

                            if self.organize_enabled {
                                ui.label(lang.tr("Шаблон подпапок:", "Subfolder template:"));
                                needs_recalc |= ui
                                    .add(
                                        egui::TextEdit::singleline(&mut self.organize_template)
                                            .hint_text("{year}/{month}"),
                                    )
                                    .changed();
                                ui.horizontal_wrapped(|ui| {
                                    for pat in [
                                        "{year}/{month}",
                                        "{exif_date}",
                                        "{media_date}/{res}",
                                        "{artist}/{album}",
                                    ] {
                                        if ui.small_button(pat).clicked() {
                                            self.organize_template = pat.to_string();
                                            needs_recalc = true;
                                        }
                                    }
                                });
                            }

                            ui.add_space(4.0);
                            ui.collapsing(
                                lang.tr("⚙ Порядок этапов (Pipeline)", "⚙ Rule Pipeline Order"),
                                |ui| {
                                    let len = self.rule_order.len();
                                    let mut swap_pair: Option<(usize, usize)> = None;
                                    for i in 0..len {
                                        let stage = self.rule_order[i];
                                        ui.push_id(format!("pipe_stage_{}", i), |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(format!("{}. {}", i + 1, stage.label(lang)));
                                                if ui
                                                    .add_enabled(i > 0, egui::Button::new("▲").small())
                                                    .clicked()
                                                {
                                                    swap_pair = Some((i, i - 1));
                                                }
                                                if ui
                                                    .add_enabled(
                                                        i + 1 < len,
                                                        egui::Button::new("▼").small(),
                                                    )
                                                    .clicked()
                                                {
                                                    swap_pair = Some((i, i + 1));
                                                }
                                            });
                                        });
                                    }
                                    if let Some((a, b)) = swap_pair {
                                        self.rule_order.swap(a, b);
                                        needs_recalc = true;
                                    }
                                    if ui
                                        .small_button(lang.tr("↺ По умолчанию", "↺ Reset order"))
                                        .clicked()
                                    {
                                        self.rule_order = default_rule_order();
                                        needs_recalc = true;
                                    }
                                },
                            );
                        });
                        ui.add_space(6.0);
                    });
            });

        // --- ЦЕНТРАЛЬНАЯ ТАБЛИЦА ПРЕДПРОСМОТРА ---
        egui::CentralPanel::default().show(ctx, |ui| {
            if settings_dirty {
                self.save_settings();
            }
            if trigger_update_check_now {
                self.trigger_update_check(ctx);
            }
            if trigger_dup_scan {
                self.start_duplicate_scan(ctx);
            }
            if let Some(target) = single_trash_target.take() {
                self.delete_paths_to_trash(vec![target], ctx);
            }
            if let Some(folder) = navigate_to.take() {
                self.open_new_directory(folder, ctx);
            } else if needs_reload {
                if let Some(dir) = self.current_dir.clone() {
                    self.load_directory(dir, ctx);
                }
            } else if needs_recalc {
                self.recalculate_preview();
            }

            if trigger_reset {
                self.reset_rules();
            }
            if trigger_apply && can_apply_now {
                self.apply_rename(ctx);
            }
            if trigger_undo {
                self.undo_rename(ctx);
            }

            ui.horizontal_wrapped(|ui| {
                ui.heading(lang.tr("Файлы и папки для переименования", "Files to Rename"));
                ui.weak(lang.tr(
                    "• Клик — превью • ✏ — правка строки • Двойной клик — открыть",
                    "• Click to preview • ✏ edit row • Double-click to open",
                ));
            });
            ui.add_space(4.0);

            let mut toggle_all: Option<bool> = None;
            let mut toggled_item: Option<(PathBuf, bool)> = None;
            let mut start_inline_edit: Option<(PathBuf, String)> = None;
            let mut commit_inline_edit: Option<(PathBuf, String)> = None;
            let mut cancel_inline_edit = false;
            let mut clear_manual_override: Option<PathBuf> = None;

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
            let label_video = lang.tr("🎬 Видео", "🎬 Video");
            let label_audio = lang.tr("🎵 Аудио", "🎵 Audio");
            let label_sidecar = lang.tr("📎 Парный", "📎 Sidecar");
            let label_file = lang.tr("📄 Файл", "📄 File");

            let current_selected = self.selected_path.clone();
            let use_diff = self.inline_diff;
            let editing_path_cur = self.editing_row_path.clone();
            let mut editing_buf = self.editing_row_text.clone();

            card_frame(is_dark).show(ui, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(34.0))
                    .column(Column::initial(95.0).resizable(true))
                    .column(Column::initial(215.0).resizable(true))
                    .column(Column::initial(250.0).resizable(true))
                    .column(Column::initial(105.0).resizable(true))
                    .column(Column::initial(80.0).resizable(true))
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
                                } else if item.is_sidecar {
                                    label_sidecar
                                } else if item.is_video {
                                    label_video
                                } else if item.is_audio {
                                    label_audio
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
                                } else if item.is_video {
                                    let video_col = if is_dark {
                                        egui::Color32::from_rgb(95, 195, 255)
                                    } else {
                                        egui::Color32::from_rgb(15, 105, 190)
                                    };
                                    ui.colored_label(video_col, kind_str);
                                } else if item.is_audio {
                                    let audio_col = if is_dark {
                                        egui::Color32::from_rgb(195, 145, 255)
                                    } else {
                                        egui::Color32::from_rgb(125, 50, 190)
                                    };
                                    ui.colored_label(audio_col, kind_str);
                                } else if item.is_sidecar {
                                    let sc_col = if is_dark {
                                        egui::Color32::from_rgb(120, 225, 180)
                                    } else {
                                        egui::Color32::from_rgb(20, 130, 90)
                                    };
                                    ui.colored_label(sc_col, kind_str);
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
                                if editing_path_cur.as_ref() == Some(&item.old_path) {
                                    ui.horizontal(|ui| {
                                        let ed = ui.add(
                                            egui::TextEdit::singleline(&mut editing_buf)
                                                .desired_width((ui.available_width() - 48.0).max(90.0)),
                                        );
                                        if ed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            commit_inline_edit = Some((item.old_path.clone(), editing_buf.clone()));
                                        }
                                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                            cancel_inline_edit = true;
                                        }
                                        if ui.small_button("✔").clicked() {
                                            commit_inline_edit = Some((item.old_path.clone(), editing_buf.clone()));
                                        }
                                        if ui.small_button("✖").clicked() {
                                            cancel_inline_edit = true;
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        if !item.included {
                                            ui.colored_label(muted, &item.old_name);
                                        } else {
                                            if self.organize_enabled
                                                && item.target_rel_dir != item.rel_parent
                                                && item.target_rel_dir != "."
                                            {
                                                ui.weak(format!("{}\\", item.target_rel_dir));
                                            }
                                            if item.issue != RowIssue::None {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(235, 80, 70),
                                                    &item.new_name,
                                                );
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
                                                    ui.strong(&item.new_name);
                                                }
                                            } else {
                                                ui.label(&item.new_name);
                                            }

                                            if ui
                                                .small_button("✏")
                                                .on_hover_text(lang.tr(
                                                    "Редактировать новое имя вручную",
                                                    "Edit new name manually",
                                                ))
                                                .clicked()
                                            {
                                                start_inline_edit =
                                                    Some((item.old_path.clone(), item.new_name.clone()));
                                            }
                                            if item.has_manual_override
                                                && ui
                                                    .small_button("✖")
                                                    .on_hover_text(lang.tr(
                                                        "Сбросить ручную правку",
                                                        "Clear manual override",
                                                    ))
                                                    .clicked()
                                            {
                                                clear_manual_override = Some(item.old_path.clone());
                                            }
                                        }
                                    });
                                }
                            });

                            row.col(|ui| {
                                if item.issue != RowIssue::None {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(235, 80, 70),
                                        egui::RichText::new(item.issue.label(lang)).strong(),
                                    );
                                } else if !item.included {
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

            if editing_path_cur.is_some() {
                self.editing_row_text = editing_buf;
            }
            if let Some((p, txt)) = start_inline_edit {
                self.editing_row_path = Some(p);
                self.editing_row_text = txt;
            } else if let Some((p, txt)) = commit_inline_edit {
                self.editing_row_path = None;
                let clean = txt.trim().to_string();
                if !clean.is_empty() {
                    self.manual_overrides.insert(p, clean);
                } else {
                    self.manual_overrides.remove(&p);
                }
                self.recalculate_preview();
            } else if cancel_inline_edit {
                self.editing_row_path = None;
            } else if let Some(p) = clear_manual_override {
                self.manual_overrides.remove(&p);
                self.recalculate_preview();
            }

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
