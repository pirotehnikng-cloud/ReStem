use crate::audio_info::AudioMediaInfo;
use crate::hash_utils;
use crate::media_info::VideoMediaInfo;
use crate::photo_meta::PhotoMeta;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub fn tr(self, ru: &'static str, en: &'static str) -> &'static str {
        match self {
            Lang::Ru => ru,
            Lang::En => en,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SortField {
    NaturalName,
    AlphaName,
    ModifiedDate,
    Size,
    Extension,
    FolderPath,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum StatusFilter {
    All,
    OnlyChanged,
    OnlyIncluded,
    OnlyUnchanged,
    OnlyIssues,
}

#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum SizeFilterMode {
    #[default]
    Any,
    GreaterThanKB,
    LessThanKB,
    BetweenKB,
}

#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum CaseMode {
    #[default]
    None,
    Lower,
    Upper,
    Capitalize,
    Title,
}

#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum TranslitMode {
    #[default]
    None,
    RuToLat,
    LatToRu,
}

#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum NumMode {
    #[default]
    None,
    Suffix,
    Prefix,
}

#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum ExtMode {
    #[default]
    Keep,
    Lower,
    Upper,
    Remove,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaceStep {
    #[serde(default)]
    pub find: String,
    #[serde(default)]
    pub replace: String,
    #[serde(default)]
    pub use_regex: bool,
    #[serde(default = "default_true")]
    pub case_sensitive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleStage {
    Trim,
    FindReplace,
    InsertAtPos,
    PrefixSuffix,
    Translit,
    Sanitize,
    Case,
    Numbering,
}

impl RuleStage {
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            RuleStage::Trim => lang.tr("Обрезка краёв", "Trim edges"),
            RuleStage::FindReplace => lang.tr("Поиск и замена", "Find & Replace"),
            RuleStage::InsertAtPos => lang.tr("Вставка по позиции", "Insert at index"),
            RuleStage::PrefixSuffix => lang.tr("Префикс и Суффикс", "Prefix & Suffix"),
            RuleStage::Translit => lang.tr("Транслитерация", "Transliteration"),
            RuleStage::Sanitize => lang.tr("Чистка имён", "Sanitize names"),
            RuleStage::Case => lang.tr("Смена регистра", "Case transform"),
            RuleStage::Numbering => lang.tr("Автонумерация", "Numbering"),
        }
    }
}

pub fn default_rule_order() -> Vec<RuleStage> {
    vec![
        RuleStage::Trim,
        RuleStage::FindReplace,
        RuleStage::InsertAtPos,
        RuleStage::PrefixSuffix,
        RuleStage::Translit,
        RuleStage::Sanitize,
        RuleStage::Case,
        RuleStage::Numbering,
    ]
}

fn default_true() -> bool {
    true
}
fn default_num_start() -> usize {
    1
}
fn default_num_step() -> usize {
    1
}
fn default_num_pad() -> usize {
    2
}
fn default_num_sep() -> String {
    "_".to_string()
}
fn default_size_min() -> u64 {
    100
}
fn default_size_max() -> u64 {
    10240
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowIssue {
    None,
    Duplicate,
    ReservedName,
    InvalidChars,
    PathTooLong,
}

impl RowIssue {
    pub fn label(&self, lang: Lang) -> &'static str {
        match self {
            RowIssue::None => "",
            RowIssue::Duplicate => lang.tr("⚠ Дубликат", "⚠ Duplicate"),
            RowIssue::ReservedName => lang.tr("⚠ Запрещ. имя", "⚠ Reserved name"),
            RowIssue::InvalidChars => lang.tr("⚠ Недоп. символы", "⚠ Invalid chars"),
            RowIssue::PathTooLong => lang.tr("⚠ Путь >260", "⚠ Path >260"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullPreset {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub find_str: String,
    #[serde(default)]
    pub replace_str: String,
    #[serde(default)]
    pub use_regex: bool,
    #[serde(default = "default_true")]
    pub case_sensitive: bool,
    #[serde(default)]
    pub extra_replaces: Vec<ReplaceStep>,
    #[serde(default = "default_rule_order")]
    pub rule_order: Vec<RuleStage>,
    #[serde(default)]
    pub case_mode: CaseMode,
    #[serde(default)]
    pub translit_mode: TranslitMode,
    #[serde(default)]
    pub sanitize_names: bool,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub trim_start: usize,
    #[serde(default)]
    pub trim_end: usize,
    #[serde(default)]
    pub insert_text: String,
    #[serde(default)]
    pub insert_pos: usize,
    #[serde(default)]
    pub insert_from_end: bool,
    #[serde(default)]
    pub num_mode: NumMode,
    #[serde(default = "default_num_start")]
    pub num_start: usize,
    #[serde(default = "default_num_step")]
    pub num_step: usize,
    #[serde(default = "default_num_pad")]
    pub num_pad: usize,
    #[serde(default = "default_num_sep")]
    pub num_sep: String,
    #[serde(default = "default_true")]
    pub reset_num_per_dir: bool,
    #[serde(default)]
    pub ext_mode: ExtMode,
    #[serde(default)]
    pub new_ext: String,
    #[serde(default)]
    pub ext_filter: String,
    #[serde(default)]
    pub ext_exclude_filter: String,
    #[serde(default)]
    pub exclude_words_filter: String,
    #[serde(default)]
    pub size_filter_mode: SizeFilterMode,
    #[serde(default = "default_size_min")]
    pub size_min_kb: u64,
    #[serde(default = "default_size_max")]
    pub size_max_kb: u64,
    #[serde(default)]
    pub auto_suffix_duplicates: bool,
    #[serde(default)]
    pub sync_sidecars: bool,
    #[serde(default)]
    pub organize_enabled: bool,
    #[serde(default)]
    pub organize_template: String,
}

impl Default for FullPreset {
    fn default() -> Self {
        Self {
            title: String::new(),
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
            num_mode: NumMode::None,
            num_start: 1,
            num_step: 1,
            num_pad: 2,
            num_sep: "_".to_string(),
            reset_num_per_dir: true,
            ext_mode: ExtMode::Keep,
            new_ext: String::new(),
            ext_filter: String::new(),
            ext_exclude_filter: String::new(),
            exclude_words_filter: String::new(),
            size_filter_mode: SizeFilterMode::Any,
            size_min_kb: 100,
            size_max_kb: 10240,
            auto_suffix_duplicates: false,
            sync_sidecars: false,
            organize_enabled: false,
            organize_template: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct FsEntry {
    pub path: PathBuf,
    pub parent_dir: PathBuf,
    pub parent_name: String,
    pub rel_parent: String,
    pub name: String,
    pub ext_lower: String,
    pub is_dir: bool,
    pub is_image: bool,
    pub is_video: bool,
    pub is_audio: bool,
    pub is_hidden: bool,
    pub depth: usize,
    pub size: u64,
    pub modified_ts: u64,
    pub photo_meta: Option<PhotoMeta>,
    pub video_info: Option<VideoMediaInfo>,
    pub audio_info: Option<AudioMediaInfo>,
}

#[derive(Clone)]
pub struct PreviewRow {
    pub old_path: PathBuf,
    pub parent_dir: PathBuf,
    pub rel_parent: String,
    pub is_dir: bool,
    pub is_video: bool,
    pub is_audio: bool,
    pub is_sidecar: bool,
    pub depth: usize,
    pub size: u64,
    pub modified_ts: u64,
    pub old_name: String,
    pub new_name: String,
    pub target_rel_dir: String,
    pub target_path: PathBuf,
    pub included: bool,
    pub changed: bool,
    pub issue: RowIssue,
    pub has_manual_override: bool,
}

#[derive(Clone, Debug)]
pub struct UndoBatch {
    pub moves: Vec<(PathBuf, PathBuf)>, // (current_new_path, original_old_path)
    pub created_dirs: Vec<PathBuf>,
}

pub type HashCache = HashMap<PathBuf, (Option<String>, Option<String>)>;

pub struct RenameConfig {
    pub sort_field: SortField,
    pub sort_asc: bool,
    pub folders_first: bool,
    pub search_query: String,
    pub search_regex: bool,
    pub search_case_sensitive: bool,
    pub status_filter: StatusFilter,
    pub include_files: bool,
    pub include_dirs: bool,
    pub hide_system_hidden: bool,
    pub ext_filter: String,
    pub ext_exclude_filter: String,
    pub exclude_words_filter: String,
    pub size_filter_mode: SizeFilterMode,
    pub size_min_kb: u64,
    pub size_max_kb: u64,
    pub find_str: String,
    pub replace_str: String,
    pub use_regex: bool,
    pub case_sensitive: bool,
    pub extra_replaces: Vec<ReplaceStep>,
    pub rule_order: Vec<RuleStage>,
    pub case_mode: CaseMode,
    pub translit_mode: TranslitMode,
    pub sanitize_names: bool,
    pub prefix: String,
    pub suffix: String,
    pub trim_start: usize,
    pub trim_end: usize,
    pub insert_text: String,
    pub insert_pos: usize,
    pub insert_from_end: bool,
    pub use_custom_list: bool,
    pub custom_list_raw: String,
    pub num_mode: NumMode,
    pub num_start: usize,
    pub num_step: usize,
    pub num_pad: usize,
    pub num_sep: String,
    pub reset_num_per_dir: bool,
    pub ext_mode: ExtMode,
    pub new_ext: String,
    pub auto_suffix_duplicates: bool,
    pub sync_sidecars: bool,
    pub organize_enabled: bool,
    pub organize_template: String,
}

impl From<&FullPreset> for RenameConfig {
    fn from(p: &FullPreset) -> Self {
        Self {
            sort_field: SortField::NaturalName,
            sort_asc: true,
            folders_first: true,
            search_query: String::new(),
            search_regex: false,
            search_case_sensitive: false,
            status_filter: StatusFilter::All,
            include_files: true,
            include_dirs: true,
            hide_system_hidden: true,
            ext_filter: p.ext_filter.clone(),
            ext_exclude_filter: p.ext_exclude_filter.clone(),
            exclude_words_filter: p.exclude_words_filter.clone(),
            size_filter_mode: p.size_filter_mode,
            size_min_kb: p.size_min_kb,
            size_max_kb: p.size_max_kb,
            find_str: p.find_str.clone(),
            replace_str: p.replace_str.clone(),
            use_regex: p.use_regex,
            case_sensitive: p.case_sensitive,
            extra_replaces: p.extra_replaces.clone(),
            rule_order: if p.rule_order.is_empty() {
                default_rule_order()
            } else {
                p.rule_order.clone()
            },
            case_mode: p.case_mode,
            translit_mode: p.translit_mode,
            sanitize_names: p.sanitize_names,
            prefix: p.prefix.clone(),
            suffix: p.suffix.clone(),
            trim_start: p.trim_start,
            trim_end: p.trim_end,
            insert_text: p.insert_text.clone(),
            insert_pos: p.insert_pos,
            insert_from_end: p.insert_from_end,
            use_custom_list: false,
            custom_list_raw: String::new(),
            num_mode: p.num_mode,
            num_start: p.num_start,
            num_step: p.num_step,
            num_pad: p.num_pad,
            num_sep: p.num_sep.clone(),
            reset_num_per_dir: p.reset_num_per_dir,
            ext_mode: p.ext_mode,
            new_ext: p.new_ext.clone(),
            auto_suffix_duplicates: p.auto_suffix_duplicates,
            sync_sidecars: p.sync_sidecars,
            organize_enabled: p.organize_enabled,
            organize_template: p.organize_template.clone(),
        }
    }
}

pub fn is_sidecar_ext(ext: &str) -> bool {
    matches!(
        ext,
        "srt" | "ass" | "ssa" | "vtt" | "sub" | "idx" | "nfo" | "xmp" | "thm" | "aae"
    )
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
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

pub fn decompose_timestamp(secs: u64) -> (i64, u32, u32, u64, u64) {
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

pub fn format_timestamp(secs: u64) -> String {
    if secs == 0 {
        return "—".to_string();
    }
    let (year, m, d, hours, mins) = decompose_timestamp(secs);
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, m, d, hours, mins)
}

pub fn transliterate_ru_to_lat(input: &str) -> String {
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

pub fn transliterate_lat_to_ru(input: &str) -> String {
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

pub fn sanitize_stem(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut prev_space = false;

    for ch in input.chars() {
        if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()
        {
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

pub fn expand_smart_tags(template: &str, entry: &FsEntry, hash_cache: &mut HashCache) -> String {
    if !template.contains('{') {
        return template.to_string();
    }
    let (year, month, day, _, _) = decompose_timestamp(entry.modified_ts);
    let date_s = format!("{:04}-{:02}-{:02}", year, month, day);
    let year_s = format!("{:04}", year);
    let month_s = format!("{:02}", month);
    let day_s = format!("{:02}", day);
    let size_s = if entry.is_dir {
        String::new()
    } else {
        hash_utils::format_size_tag(entry.size)
    };

    let (crc32_s, sha256_8_s) = if !entry.is_dir
        && (template.contains("{crc32}") || template.contains("{sha256_8}"))
    {
        let cached = hash_cache.entry(entry.path.clone()).or_insert((None, None));
        if template.contains("{crc32}") && cached.0.is_none() {
            cached.0 = hash_utils::file_crc32_hex(&entry.path);
        }
        if template.contains("{sha256_8}") && cached.1.is_none() {
            cached.1 = hash_utils::file_sha256_8(&entry.path);
        }
        (
            cached.0.clone().unwrap_or_default(),
            cached.1.clone().unwrap_or_default(),
        )
    } else {
        (String::new(), String::new())
    };

    let (exif_d, exif_t, cam_s, lens_s, iso_s, aperture_s, shutter_s) =
        if let Some(pm) = entry.photo_meta.as_ref() {
            (
                pm.exif_date.as_deref().unwrap_or(&date_s).to_string(),
                pm.exif_time.clone().unwrap_or_default(),
                pm.camera.clone().unwrap_or_default(),
                pm.lens.clone().unwrap_or_default(),
                pm.iso_tag().unwrap_or_default(),
                pm.aperture_tag().unwrap_or_default(),
                pm.shutter_tag().unwrap_or_default(),
            )
        } else {
            (
                date_s.clone(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        };

    let dims = entry
        .photo_meta
        .as_ref()
        .and_then(|p| p.dimensions())
        .or_else(|| entry.video_info.as_ref().and_then(|v| v.dimensions()));
    let (w_s, h_s, wxh_s) = if let Some((w, h)) = dims {
        (w.to_string(), h.to_string(), format!("{}x{}", w, h))
    } else {
        (String::new(), String::new(), String::new())
    };

    let (
        res_s,
        vcodec_s,
        acodec_s,
        ach_s,
        fps_s,
        vid_dur_s,
        bitrate_s,
        hdr_s,
        media_date_s,
    ) = if let Some(v) = entry.video_info.as_ref() {
        (
            v.resolution_label().unwrap_or_default(),
            v.video_codec.clone().unwrap_or_default(),
            v.audio_codec.clone().unwrap_or_default(),
            v.audio_channels_tag().unwrap_or_default(),
            v.fps_tag().unwrap_or_default(),
            v.duration_tag().unwrap_or_default(),
            v.bitrate_tag().unwrap_or_default(),
            v.hdr.clone().unwrap_or_default(),
            v.recorded_date.as_deref().unwrap_or(&date_s).to_string(),
        )
    } else {
        (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            date_s.clone(),
        )
    };

    let (
        artist_s,
        album_s,
        title_s,
        track_s,
        audio_year_s,
        genre_s,
        audio_br_s,
        sample_rate_s,
        aud_dur_s,
    ) = if let Some(a) = entry.audio_info.as_ref() {
        (
            a.artist.clone().unwrap_or_default(),
            a.album.clone().unwrap_or_default(),
            a.title.clone().unwrap_or_default(),
            a.track_tag().unwrap_or_default(),
            a.year.clone().unwrap_or_default(),
            a.genre.clone().unwrap_or_default(),
            a.bitrate_tag().unwrap_or_default(),
            a.sample_rate_tag().unwrap_or_default(),
            a.duration_tag().unwrap_or_default(),
        )
    } else {
        (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
    };

    let dur_s = if !vid_dur_s.is_empty() {
        vid_dur_s
    } else {
        aud_dur_s
    };

    template
        .replace("{date}", &date_s)
        .replace("{year}", &year_s)
        .replace("{month}", &month_s)
        .replace("{day}", &day_s)
        .replace("{parent}", &entry.parent_name)
        .replace("{size}", &size_s)
        .replace("{crc32}", &crc32_s)
        .replace("{sha256_8}", &sha256_8_s)
        .replace("{exif_date}", &exif_d)
        .replace("{exif_time}", &exif_t)
        .replace("{camera}", &cam_s)
        .replace("{lens}", &lens_s)
        .replace("{iso}", &iso_s)
        .replace("{aperture}", &aperture_s)
        .replace("{shutter}", &shutter_s)
        .replace("{w}x{h}", &wxh_s)
        .replace("{w}", &w_s)
        .replace("{h}", &h_s)
        .replace("{res}", &res_s)
        .replace("{vcodec}", &vcodec_s)
        .replace("{acodec}", &acodec_s)
        .replace("{ach}", &ach_s)
        .replace("{fps}", &fps_s)
        .replace("{duration}", &dur_s)
        .replace("{bitrate}", &bitrate_s)
        .replace("{hdr}", &hdr_s)
        .replace("{media_date}", &media_date_s)
        .replace("{artist}", &artist_s)
        .replace("{album}", &album_s)
        .replace("{title}", &title_s)
        .replace("{track}", &track_s)
        .replace("{audio_year}", &audio_year_s)
        .replace("{genre}", &genre_s)
        .replace("{audio_bitrate}", &audio_br_s)
        .replace("{sample_rate}", &sample_rate_s)
}

fn is_windows_reserved_stem(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim();
    let up = stem.to_ascii_uppercase();
    matches!(
        up.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

pub fn expand_organize_rel_dir(
    template: &str,
    entry: &FsEntry,
    hash_cache: &mut HashCache,
) -> PathBuf {
    let expanded = expand_smart_tags(template, entry, hash_cache);
    let mut rel = PathBuf::new();
    for raw_seg in expanded.split(['/', '\\']) {
        let mut cleaned = sanitize_stem(raw_seg);
        if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
            continue;
        }
        if is_windows_reserved_stem(&cleaned) {
            cleaned.push('_');
        }
        rel.push(cleaned);
    }
    rel
}

fn validate_target_path(new_name: &str, target_path: &Path) -> RowIssue {
    if new_name.is_empty()
        || new_name == "."
        || new_name == ".."
        || new_name.ends_with('.')
        || new_name.ends_with(' ')
        || new_name.chars().any(|ch| {
            matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()
        })
    {
        return RowIssue::InvalidChars;
    }
    if is_windows_reserved_stem(new_name) {
        return RowIssue::ReservedName;
    }
    if target_path.to_string_lossy().chars().count() > 259 {
        return RowIssue::PathTooLong;
    }
    RowIssue::None
}

fn append_collision_suffix(name: &str, is_dir: bool, n: usize) -> String {
    if is_dir {
        return format!("{}_{}", name, n);
    }
    if let Some(dot) = name.rfind('.') {
        if dot > 0 {
            return format!("{}_{}{}", &name[..dot], n, &name[dot..]);
        }
    }
    format!("{}_{}", name, n)
}

fn apply_find_replace_step(
    stem: &str,
    find: &str,
    replace: &str,
    use_regex: bool,
    case_sensitive: bool,
    entry: &FsEntry,
    hash_cache: &mut HashCache,
) -> String {
    if find.is_empty() {
        return stem.to_string();
    }
    let pattern = if use_regex {
        find.to_string()
    } else {
        regex::escape(find)
    };
    if let Ok(re) = RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .build()
    {
        let rep_expanded = expand_smart_tags(replace, entry, hash_cache);
        re.replace_all(stem, rep_expanded.as_str()).to_string()
    } else {
        stem.to_string()
    }
}

pub fn split_stem_and_ext(name: &str, is_dir: bool) -> (String, String) {
    if is_dir {
        (name.to_string(), String::new())
    } else {
        let p = Path::new(name);
        let s = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let e = p
            .extension()
            .map(|ext_os| ext_os.to_string_lossy().to_string())
            .unwrap_or_default();
        (s, e)
    }
}

pub fn compute_new_name(
    cfg: &RenameConfig,
    entry: &FsEntry,
    idx: usize,
    custom_list_override: Option<&str>,
    hash_cache: &mut HashCache,
) -> String {
    let (mut stem, raw_ext) = split_stem_and_ext(&entry.name, entry.is_dir);

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

    let order = if cfg.rule_order.is_empty() {
        default_rule_order()
    } else {
        cfg.rule_order.clone()
    };

    for stage in order {
        match stage {
            RuleStage::Trim => {
                let chars: Vec<char> = stem.chars().collect();
                let start = cfg.trim_start.min(chars.len());
                let end = chars.len().saturating_sub(cfg.trim_end).max(start);
                stem = chars[start..end].iter().collect();
            }
            RuleStage::FindReplace => {
                stem = apply_find_replace_step(
                    &stem,
                    &cfg.find_str,
                    &cfg.replace_str,
                    cfg.use_regex,
                    cfg.case_sensitive,
                    entry,
                    hash_cache,
                );
                for step in &cfg.extra_replaces {
                    stem = apply_find_replace_step(
                        &stem,
                        &step.find,
                        &step.replace,
                        step.use_regex,
                        step.case_sensitive,
                        entry,
                        hash_cache,
                    );
                }
            }
            RuleStage::InsertAtPos => {
                if !cfg.insert_text.is_empty() {
                    let ins_expanded = expand_smart_tags(&cfg.insert_text, entry, hash_cache);
                    let current_chars: Vec<char> = stem.chars().collect();
                    let idx_pos = if cfg.insert_from_end {
                        current_chars
                            .len()
                            .saturating_sub(cfg.insert_pos.min(current_chars.len()))
                    } else {
                        cfg.insert_pos.min(current_chars.len())
                    };
                    let left: String = current_chars[..idx_pos].iter().collect();
                    let right: String = current_chars[idx_pos..].iter().collect();
                    stem = format!("{}{}{}", left, ins_expanded, right);
                }
            }
            RuleStage::PrefixSuffix => {
                let pref_exp = expand_smart_tags(&cfg.prefix, entry, hash_cache);
                let suff_exp = expand_smart_tags(&cfg.suffix, entry, hash_cache);
                stem = format!("{}{}{}", pref_exp, stem, suff_exp);
            }
            RuleStage::Translit => {
                stem = match cfg.translit_mode {
                    TranslitMode::None => stem,
                    TranslitMode::RuToLat => transliterate_ru_to_lat(&stem),
                    TranslitMode::LatToRu => transliterate_lat_to_ru(&stem),
                };
            }
            RuleStage::Sanitize => {
                if cfg.sanitize_names {
                    stem = sanitize_stem(&stem);
                }
            }
            RuleStage::Case => {
                stem = match cfg.case_mode {
                    CaseMode::None => stem,
                    CaseMode::Lower => stem.to_lowercase(),
                    CaseMode::Upper => stem.to_uppercase(),
                    CaseMode::Capitalize => {
                        let mut c = stem.chars();
                        match c.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().collect::<String>()
                                    + &c.as_str().to_lowercase()
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
                                    f.to_uppercase().collect::<String>()
                                        + &c.as_str().to_lowercase()
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" "),
                };
            }
            RuleStage::Numbering => {
                if cfg.num_mode != NumMode::None {
                    let num = cfg.num_start + idx * cfg.num_step.max(1);
                    let num_str = format!("{:0width$}", num, width = cfg.num_pad.max(1));
                    stem = match cfg.num_mode {
                        NumMode::Suffix if !stem.is_empty() => {
                            format!("{}{}{}", stem, cfg.num_sep, num_str)
                        }
                        NumMode::Prefix if !stem.is_empty() => {
                            format!("{}{}{}", num_str, cfg.num_sep, stem)
                        }
                        _ => num_str,
                    };
                }
            }
        }
    }

    let final_ext = compute_final_ext(cfg, &raw_ext, entry.is_dir);
    format!("{}{}", stem, final_ext)
}

fn compute_final_ext(cfg: &RenameConfig, raw_ext: &str, is_dir: bool) -> String {
    if is_dir {
        return String::new();
    }
    let custom_clean = cfg.new_ext.trim().trim_start_matches('.');
    let mut ext_val = if !custom_clean.is_empty() {
        custom_clean.to_string()
    } else {
        raw_ext.to_string()
    };

    if cfg.translit_mode == TranslitMode::RuToLat {
        ext_val = transliterate_ru_to_lat(&ext_val);
    }

    ext_val = match cfg.ext_mode {
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
}

pub fn compare_entries(cfg: &RenameConfig, a: &FsEntry, b: &FsEntry) -> Ordering {
    if cfg.folders_first && a.is_dir != b.is_dir {
        return b.is_dir.cmp(&a.is_dir);
    }

    let base_ord = match cfg.sort_field {
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

    if cfg.sort_asc {
        base_ord
    } else {
        base_ord.reverse()
    }
}

pub struct PreviewSummary {
    pub rows: Vec<PreviewRow>,
    pub total_filtered: usize,
    pub included_count: usize,
    pub changed_count: usize,
    pub issue_count: usize,
}

pub fn compute_preview(
    root_dir: Option<&Path>,
    all_entries: &[FsEntry],
    excluded_paths: &HashSet<PathBuf>,
    manual_overrides: &HashMap<PathBuf, String>,
    cfg: &RenameConfig,
    hash_cache: &mut HashCache,
) -> PreviewSummary {
    let allowed_exts: Vec<String> = cfg
        .ext_filter
        .split(',')
        .map(|s| s.trim().trim_start_matches('.').to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let excluded_exts: Vec<String> = cfg
        .ext_exclude_filter
        .split(',')
        .map(|s| s.trim().trim_start_matches('.').to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let excluded_words: Vec<String> = cfg
        .exclude_words_filter
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let search_trimmed = cfg.search_query.trim();
    let search_re = if !search_trimmed.is_empty() {
        let pat = if cfg.search_regex {
            search_trimmed.to_string()
        } else {
            regex::escape(search_trimmed)
        };
        RegexBuilder::new(&pat)
            .case_insensitive(!cfg.search_case_sensitive)
            .build()
            .ok()
    } else {
        None
    };

    let mut filtered: Vec<&FsEntry> = all_entries
        .iter()
        .filter(|e| {
            if cfg.hide_system_hidden && e.is_hidden {
                return false;
            }
            if e.is_dir && !cfg.include_dirs {
                return false;
            }
            if !e.is_dir && !cfg.include_files {
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
            if !allowed_exts.is_empty() && (e.is_dir || !allowed_exts.contains(&e.ext_lower)) {
                return false;
            }
            if !excluded_exts.is_empty() && !e.is_dir && excluded_exts.contains(&e.ext_lower) {
                return false;
            }
            if !e.is_dir {
                let min_b = cfg.size_min_kb.saturating_mul(1024);
                let max_b = cfg.size_max_kb.saturating_mul(1024);
                match cfg.size_filter_mode {
                    SizeFilterMode::Any => {}
                    SizeFilterMode::GreaterThanKB if e.size < min_b => return false,
                    SizeFilterMode::LessThanKB if e.size > max_b => return false,
                    SizeFilterMode::BetweenKB if e.size < min_b || e.size > max_b => return false,
                    _ => {}
                }
            }
            true
        })
        .collect();

    filtered.sort_by(|a, b| compare_entries(cfg, a, b));
    let total_filtered = filtered.len();

    let custom_lines: Vec<&str> = if cfg.use_custom_list {
        cfg.custom_list_raw
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    let mut dir_counters: HashMap<PathBuf, usize> = HashMap::new();
    let mut global_counter: usize = 0;

    struct DraftRow {
        entry_idx: usize,
        included: bool,
        new_name: String,
        target_dir: PathBuf,
        target_rel_dir: String,
        is_sidecar: bool,
        has_manual_override: bool,
    }

    let mut drafts: Vec<DraftRow> = Vec::with_capacity(filtered.len());
    let mut primary_map: HashMap<(PathBuf, String), (String, PathBuf, String)> = HashMap::new();

    for (entry_idx, entry) in filtered.iter().enumerate() {
        let included = !excluded_paths.contains(&entry.path);
        let manual = manual_overrides.get(&entry.path);
        let is_sidecar = !entry.is_dir && is_sidecar_ext(&entry.ext_lower);

        let (new_name, target_dir, target_rel_dir) = if included {
            let list_override = if cfg.use_custom_list {
                custom_lines.get(global_counter).copied()
            } else {
                None
            };
            let idx = if cfg.reset_num_per_dir {
                let c = dir_counters.entry(entry.parent_dir.clone()).or_insert(0);
                let current = *c;
                *c += 1;
                current
            } else {
                global_counter
            };
            global_counter += 1;

            let computed_name = if let Some(ov) = manual {
                ov.clone()
            } else {
                compute_new_name(cfg, entry, idx, list_override, hash_cache)
            };

            let (t_dir, t_rel) = if cfg.organize_enabled
                && !entry.is_dir
                && !cfg.organize_template.trim().is_empty()
            {
                let rel_sub = expand_organize_rel_dir(&cfg.organize_template, entry, hash_cache);
                if rel_sub.as_os_str().is_empty() {
                    (entry.parent_dir.clone(), entry.rel_parent.clone())
                } else {
                    let base = root_dir.unwrap_or(&entry.parent_dir);
                    let dest = base.join(&rel_sub);
                    let rel_disp = rel_sub.display().to_string();
                    (dest, rel_disp)
                }
            } else {
                (entry.parent_dir.clone(), entry.rel_parent.clone())
            };

            if !entry.is_dir && !is_sidecar && (computed_name != entry.name || t_dir != entry.parent_dir)
            {
                let (orig_stem, _) = split_stem_and_ext(&entry.name, false);
                let (new_stem, _) = split_stem_and_ext(&computed_name, false);
                if !orig_stem.is_empty() && !new_stem.is_empty() {
                    primary_map.insert(
                        (entry.parent_dir.clone(), orig_stem.to_lowercase()),
                        (new_stem, t_dir.clone(), t_rel.clone()),
                    );
                }
            }

            (computed_name, t_dir, t_rel)
        } else {
            (
                entry.name.clone(),
                entry.parent_dir.clone(),
                entry.rel_parent.clone(),
            )
        };

        drafts.push(DraftRow {
            entry_idx,
            included,
            new_name,
            target_dir,
            target_rel_dir,
            is_sidecar: false,
            has_manual_override: manual.is_some(),
        });
    }

    // Pass 2: Sidecar matching
    if cfg.sync_sidecars && !primary_map.is_empty() {
        for d in &mut drafts {
            if !d.included || d.has_manual_override {
                continue;
            }
            let entry = filtered[d.entry_idx];
            if entry.is_dir || !is_sidecar_ext(&entry.ext_lower) {
                continue;
            }
            let (side_stem, raw_ext) = split_stem_and_ext(&entry.name, false);
            let side_low = side_stem.to_lowercase();

            let matched = if let Some(hit) = primary_map.get(&(entry.parent_dir.clone(), side_low.clone())) {
                Some((hit.0.clone(), String::new(), hit.1.clone(), hit.2.clone()))
            } else if let Some(dot_pos) = side_stem.rfind('.') {
                let base_low = side_stem[..dot_pos].to_lowercase();
                let lang_suffix = &side_stem[dot_pos..];
                primary_map
                    .get(&(entry.parent_dir.clone(), base_low))
                    .map(|hit| (hit.0.clone(), lang_suffix.to_string(), hit.1.clone(), hit.2.clone()))
            } else {
                None
            };

            if let Some((new_primary_stem, lang_suffix, t_dir, t_rel)) = matched {
                let final_ext = compute_final_ext(cfg, &raw_ext, false);
                d.new_name = format!("{}{}{}", new_primary_stem, lang_suffix, final_ext);
                d.target_dir = t_dir;
                d.target_rel_dir = t_rel;
                d.is_sidecar = true;
            }
        }
    }

    // Build moving sources key set
    let mut moving_sources: HashSet<(PathBuf, String)> = HashSet::new();
    for d in &drafts {
        let entry = filtered[d.entry_idx];
        let target_p = d.target_dir.join(&d.new_name);
        if d.included && target_p != entry.path {
            moving_sources.insert((entry.parent_dir.clone(), entry.name.to_lowercase()));
        }
    }

    // Resolve duplicates if auto_suffix_duplicates is ON, or detect collisions
    let mut issues: Vec<RowIssue> = vec![RowIssue::None; drafts.len()];

    if cfg.auto_suffix_duplicates {
        let mut used_keys: HashSet<(PathBuf, String)> = HashSet::new();
        for d in &drafts {
            let entry = filtered[d.entry_idx];
            let target_p = d.target_dir.join(&d.new_name);
            if !d.included || target_p == entry.path {
                used_keys.insert((entry.parent_dir.clone(), entry.name.to_lowercase()));
            }
        }
        for (i, d) in drafts.iter_mut().enumerate() {
            let entry = filtered[d.entry_idx];
            let initial_target = d.target_dir.join(&d.new_name);
            if !d.included || initial_target == entry.path {
                continue;
            }
            let base_issue = validate_target_path(&d.new_name, &initial_target);
            if base_issue != RowIssue::None {
                issues[i] = base_issue;
                continue;
            }
            let mut candidate = d.new_name.clone();
            let mut suffix_n = 1usize;
            loop {
                let key = (d.target_dir.clone(), candidate.to_lowercase());
                let cand_path = d.target_dir.join(&candidate);
                let disk_blocks = cand_path != entry.path
                    && cand_path.exists()
                    && !moving_sources.contains(&key);
                if !used_keys.contains(&key) && !disk_blocks {
                    used_keys.insert(key);
                    break;
                }
                candidate = append_collision_suffix(&d.new_name, entry.is_dir, suffix_n);
                suffix_n += 1;
            }
            d.new_name = candidate;
            let final_target = d.target_dir.join(&d.new_name);
            issues[i] = validate_target_path(&d.new_name, &final_target);
        }
    } else {
        let mut counts: HashMap<(PathBuf, String), usize> = HashMap::new();
        for d in &drafts {
            let key = (d.target_dir.clone(), d.new_name.to_lowercase());
            *counts.entry(key).or_insert(0) += 1;
        }
        for (i, d) in drafts.iter().enumerate() {
            let entry = filtered[d.entry_idx];
            let target_p = d.target_dir.join(&d.new_name);
            if !d.included || target_p == entry.path {
                continue;
            }
            let mut iss = validate_target_path(&d.new_name, &target_p);
            if iss == RowIssue::None {
                let key = (d.target_dir.clone(), d.new_name.to_lowercase());
                let dup_in_batch = counts.get(&key).copied().unwrap_or(0) > 1;
                let disk_blocks = target_p.exists() && !moving_sources.contains(&key);
                if dup_in_batch || disk_blocks {
                    iss = RowIssue::Duplicate;
                }
            }
            issues[i] = iss;
        }
    }

    let mut rows = Vec::new();
    let mut included_count = 0usize;
    let mut changed_count = 0usize;
    let mut issue_count = 0usize;

    for (i, d) in drafts.into_iter().enumerate() {
        let entry = filtered[d.entry_idx];
        let target_path = d.target_dir.join(&d.new_name);
        let changed = d.included && target_path != entry.path && !d.new_name.is_empty();
        let issue = issues[i].clone();

        let pass_status = match cfg.status_filter {
            StatusFilter::All => true,
            StatusFilter::OnlyChanged => changed,
            StatusFilter::OnlyIncluded => d.included,
            StatusFilter::OnlyUnchanged => !changed,
            StatusFilter::OnlyIssues => issue != RowIssue::None,
        };
        if !pass_status {
            continue;
        }

        if d.included {
            included_count += 1;
        }
        if changed {
            changed_count += 1;
        }
        if issue != RowIssue::None {
            issue_count += 1;
        }

        rows.push(PreviewRow {
            old_path: entry.path.clone(),
            parent_dir: entry.parent_dir.clone(),
            rel_parent: entry.rel_parent.clone(),
            is_dir: entry.is_dir,
            is_video: entry.is_video,
            is_audio: entry.is_audio,
            is_sidecar: d.is_sidecar,
            depth: entry.depth,
            size: entry.size,
            modified_ts: entry.modified_ts,
            old_name: entry.name.clone(),
            new_name: d.new_name,
            target_rel_dir: d.target_rel_dir,
            target_path,
            included: d.included,
            changed,
            issue,
            has_manual_override: d.has_manual_override,
        });
    }

    PreviewSummary {
        rows,
        total_filtered,
        included_count,
        changed_count,
        issue_count,
    }
}

fn ensure_dir_tracking(dir: &Path, created_dirs: &mut Vec<PathBuf>) -> Result<(), String> {
    if dir.exists() {
        return Ok(());
    }
    let mut missing = Vec::new();
    let mut cur = Some(dir);
    while let Some(d) = cur {
        if d.exists() {
            break;
        }
        missing.push(d.to_path_buf());
        cur = d.parent();
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    for d in missing.into_iter().rev() {
        if !created_dirs.contains(&d) {
            created_dirs.push(d);
        }
    }
    Ok(())
}

fn run_two_phase_moves(
    pairs: Vec<(PathBuf, PathBuf)>,
    created_dirs: &mut Vec<PathBuf>,
) -> (Vec<(PathBuf, PathBuf)>, Option<String>) {
    let source_keys: HashSet<(PathBuf, String)> = pairs
        .iter()
        .map(|(src, _)| {
            let par = src.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
            let nm = src
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            (par, nm)
        })
        .collect();

    let mut staged: Vec<(PathBuf, PathBuf, PathBuf)> = Vec::with_capacity(pairs.len());
    let mut completed_moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    let pid = std::process::id();

    for (idx, (src, dst)) in pairs.into_iter().enumerate() {
        let dst_par = dst.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        let dst_nm = dst
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let src_par = src.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        let src_nm = src
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        let same_case_only = src_par == dst_par && src_nm == dst_nm;
        let needs_staging = !same_case_only && (source_keys.contains(&(dst_par, dst_nm)) || dst.exists());

        if needs_staging {
            let tmp = src_par.join(format!(".restem_tmp_{}_{}", pid, idx));
            if let Err(e) = fs::rename(&src, &tmp) {
                return (
                    completed_moves,
                    Some(format!("{} -> {}: {}", src.display(), tmp.display(), e)),
                );
            }
            staged.push((tmp, src, dst));
        } else {
            staged.push((src.clone(), src, dst));
        }
    }

    for (actual_src, orig_src, dst) in staged {
        if let Some(par) = dst.parent() {
            if let Err(e) = ensure_dir_tracking(par, created_dirs) {
                return (completed_moves, Some(e));
            }
        }
        if let Err(e) = fs::rename(&actual_src, &dst) {
            return (
                completed_moves,
                Some(format!("{} -> {}: {}", orig_src.display(), dst.display(), e)),
            );
        }
        completed_moves.push((dst, orig_src));
    }

    (completed_moves, None)
}

pub fn execute_rename_batch(rows: &[PreviewRow], lang: Lang) -> Result<UndoBatch, String> {
    if let Some(bad) = rows.iter().find(|r| r.included && r.issue != RowIssue::None) {
        return Err(match lang {
            Lang::Ru => format!(
                "Исправьте проблему '{}' ({}) перед применением.",
                bad.new_name,
                bad.issue.label(lang)
            ),
            Lang::En => format!(
                "Resolve issue for '{}' ({}) before applying.",
                bad.new_name,
                bad.issue.label(lang)
            ),
        });
    }

    let mut to_rename: Vec<PreviewRow> = rows
        .iter()
        .filter(|r| r.included && r.changed)
        .cloned()
        .collect();

    if to_rename.is_empty() {
        return Ok(UndoBatch {
            moves: Vec::new(),
            created_dirs: Vec::new(),
        });
    }

    to_rename.sort_by(|a, b| {
        b.depth
            .cmp(&a.depth)
            .then_with(|| a.is_dir.cmp(&b.is_dir))
    });

    let pairs: Vec<(PathBuf, PathBuf)> = to_rename
        .into_iter()
        .map(|r| (r.old_path, r.target_path))
        .collect();

    let mut created_dirs = Vec::new();
    let (moves, err) = run_two_phase_moves(pairs, &mut created_dirs);
    if let Some(e) = err {
        if moves.is_empty() {
            return Err(e);
        }
    }

    Ok(UndoBatch {
        moves,
        created_dirs,
    })
}

pub fn execute_undo_batch(batch: UndoBatch) -> (usize, Option<String>) {
    let pairs: Vec<(PathBuf, PathBuf)> = batch.moves.into_iter().rev().collect();
    let mut dummy_dirs = Vec::new();
    let (restored, err) = run_two_phase_moves(pairs, &mut dummy_dirs);
    for dir in batch.created_dirs.into_iter().rev() {
        let _ = fs::remove_dir(&dir);
    }
    (restored.len(), err)
}

pub fn build_rollback_bat(batch: &UndoBatch) -> String {
    let mut lines = vec![
        "@echo off".to_string(),
        "chcp 65001 >nul".to_string(),
        "echo Restoring filenames via ReStem rollback script...".to_string(),
    ];
    for (curr_path, orig_path) in batch.moves.iter().rev() {
        lines.push(format!(
            "move /Y \"{}\" \"{}\"",
            curr_path.display(),
            orig_path.display()
        ));
    }
    for d in batch.created_dirs.iter().rev() {
        lines.push(format!("rmdir \"{}\" 2>nul", d.display()));
    }
    lines.push("echo Done!".to_string());
    lines.push("pause".to_string());
    lines.join("\r\n")
}

pub fn presets_json_path() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join("ReStem");
        let _ = fs::create_dir_all(&dir);
        dir.join("restem_presets.json")
    } else {
        PathBuf::from("restem_presets.json")
    }
}

pub fn load_presets_json() -> Vec<FullPreset> {
    let path = presets_json_path();
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_presets_json(presets: &[FullPreset]) {
    if let Ok(json) = serde_json::to_string_pretty(presets) {
        let _ = fs::write(presets_json_path(), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(name: &str, ext: &str) -> FsEntry {
        FsEntry {
            path: PathBuf::from(name),
            parent_dir: PathBuf::from("."),
            parent_name: "Root".to_string(),
            rel_parent: ".".to_string(),
            name: name.to_string(),
            ext_lower: ext.to_string(),
            is_dir: false,
            is_image: false,
            is_video: false,
            is_audio: false,
            is_hidden: false,
            depth: 0,
            size: 2_048,
            modified_ts: 1_795_392_000, // 2026-11-23
            photo_meta: None,
            video_info: None,
            audio_info: None,
        }
    }

    #[test]
    fn test_pipeline_order_and_multi_replace() {
        let mut cfg = RenameConfig::from(&FullPreset::default());
        let entry = sample_entry("Привет мир.txt", "txt");
        let mut cache = HashCache::new();

        cfg.find_str = "Privet".to_string();
        cfg.replace_str = "Hello".to_string();
        cfg.extra_replaces.push(ReplaceStep {
            find: "mir".to_string(),
            replace: "world".to_string(),
            use_regex: false,
            case_sensitive: false,
        });
        // Put Translit BEFORE FindReplace!
        cfg.rule_order = vec![
            RuleStage::Translit,
            RuleStage::FindReplace,
            RuleStage::Trim,
            RuleStage::InsertAtPos,
            RuleStage::PrefixSuffix,
            RuleStage::Sanitize,
            RuleStage::Case,
            RuleStage::Numbering,
        ];
        cfg.translit_mode = TranslitMode::RuToLat;

        assert_eq!(
            compute_new_name(&cfg, &entry, 0, None, &mut cache),
            "Hello world.txt"
        );
    }

    #[test]
    fn test_audio_exif_and_hash_tags() {
        let dir = std::env::temp_dir().join(format!("restem_tag_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let fpath = dir.join("song.mp3");
        fs::write(&fpath, b"123456789").unwrap();

        let mut e = sample_entry("song.mp3", "mp3");
        e.path = fpath.clone();
        e.size = 9;
        e.is_audio = true;
        e.audio_info = Some(AudioMediaInfo {
            artist: Some("Daft".to_string()),
            album: Some("RAM".to_string()),
            title: Some("Touch".to_string()),
            track: Some(7),
            year: Some("2013".to_string()),
            genre: Some("Electronic".to_string()),
            duration_secs: Some(500.0),
            bitrate_kbps: Some(320),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            format: Some("MP3".to_string()),
        });
        e.photo_meta = Some(PhotoMeta {
            width: Some(6000),
            height: Some(4000),
            exif_date: Some("2026-11-24".to_string()),
            exif_time: Some("09-10-11".to_string()),
            camera: Some("R6".to_string()),
            lens: Some("RF50".to_string()),
            iso: Some(800),
            aperture: Some(1.8),
            shutter: Some((1, 250)),
        });

        let mut cache = HashCache::new();
        let res = expand_smart_tags(
            "{track}_{artist}_{title}_{size}_{crc32}_{exif_time}_{iso}_{aperture}_{shutter}",
            &e,
            &mut cache,
        );
        assert_eq!(
            res,
            "07_Daft_Touch_9B_CBF43926_09-10-11_ISO800_f1.8_1-250s"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sidecars_validation_and_collision_resolution() {
        let dir = PathBuf::from("C:\\Videos");
        let mut vid = sample_entry("clip.mkv", "mkv");
        vid.path = dir.join("clip.mkv");
        vid.parent_dir = dir.clone();

        let mut srt = sample_entry("clip.en.srt", "srt");
        srt.path = dir.join("clip.en.srt");
        srt.parent_dir = dir.clone();

        let mut dup1 = sample_entry("a.txt", "txt");
        dup1.path = dir.join("a.txt");
        dup1.parent_dir = dir.clone();

        let mut dup2 = sample_entry("b.txt", "txt");
        dup2.path = dir.join("b.txt");
        dup2.parent_dir = dir.clone();

        let mut cfg = RenameConfig::from(&FullPreset::default());
        cfg.sync_sidecars = true;
        cfg.find_str = "clip".to_string();
        cfg.replace_str = "Vacation".to_string();

        // Check sidecar sync
        let mut cache = HashCache::new();
        let excl = HashSet::new();
        let ov = HashMap::new();
        let summary = compute_preview(Some(&dir), &[vid.clone(), srt.clone()], &excl, &ov, &cfg, &mut cache);
        let mkv_row = summary.rows.iter().find(|r| r.old_name == "clip.mkv").unwrap();
        let srt_row = summary.rows.iter().find(|r| r.old_name == "clip.en.srt").unwrap();
        assert_eq!(mkv_row.new_name, "Vacation.mkv");
        assert_eq!(srt_row.new_name, "Vacation.en.srt");
        assert!(srt_row.is_sidecar);

        // Check collision detection vs auto-suffix
        let mut cfg2 = RenameConfig::from(&FullPreset::default());
        cfg2.use_regex = true;
        cfg2.find_str = ".+".to_string();
        cfg2.replace_str = "same".to_string();
        let s_dup = compute_preview(Some(&dir), &[dup1.clone(), dup2.clone()], &excl, &ov, &cfg2, &mut cache);
        assert_eq!(s_dup.issue_count, 2);
        assert_eq!(s_dup.rows[0].issue, RowIssue::Duplicate);

        cfg2.auto_suffix_duplicates = true;
        let s_fixed = compute_preview(Some(&dir), &[dup1, dup2], &excl, &ov, &cfg2, &mut cache);
        assert_eq!(s_fixed.issue_count, 0);
        assert_eq!(s_fixed.rows[0].new_name, "same.txt");
        assert_eq!(s_fixed.rows[1].new_name, "same_1.txt");

        // Check Windows reserved name
        assert_eq!(
            validate_target_path("CON.txt", Path::new("C:\\CON.txt")),
            RowIssue::ReservedName
        );
        assert_eq!(
            validate_target_path("bad:name.txt", Path::new("C:\\bad:name.txt")),
            RowIssue::InvalidChars
        );
    }

    #[test]
    fn test_two_phase_swap_organize_and_undo() {
        let dir = std::env::temp_dir().join(format!("restem_swap_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let p_a = dir.join("a.txt");
        let p_b = dir.join("b.txt");
        fs::write(&p_a, b"ALPHA").unwrap();
        fs::write(&p_b, b"BETA").unwrap();

        let mut e_a = sample_entry("a.txt", "txt");
        e_a.path = p_a.clone();
        e_a.parent_dir = dir.clone();
        let mut e_b = sample_entry("b.txt", "txt");
        e_b.path = p_b.clone();
        e_b.parent_dir = dir.clone();

        let mut ov = HashMap::new();
        ov.insert(p_a.clone(), "b.txt".to_string());
        ov.insert(p_b.clone(), "a.txt".to_string());

        let mut cfg = RenameConfig::from(&FullPreset::default());
        cfg.organize_enabled = true;
        cfg.organize_template = "{year}/{month}".to_string();

        let mut cache = HashCache::new();
        let excl = HashSet::new();
        let sum = compute_preview(Some(&dir), &[e_a, e_b], &excl, &ov, &cfg, &mut cache);
        assert_eq!(sum.issue_count, 0);

        let batch = execute_rename_batch(&sum.rows, Lang::En).unwrap();
        assert_eq!(batch.moves.len(), 2);

        let dest_sub = dir.join("2026").join("11");
        assert_eq!(fs::read(dest_sub.join("b.txt")).unwrap(), b"ALPHA");
        assert_eq!(fs::read(dest_sub.join("a.txt")).unwrap(), b"BETA");
        assert!(!p_a.exists());

        let (restored, err) = execute_undo_batch(batch);
        assert!(err.is_none());
        assert_eq!(restored, 2);
        assert_eq!(fs::read(&p_a).unwrap(), b"ALPHA");
        assert_eq!(fs::read(&p_b).unwrap(), b"BETA");
        assert!(!dest_sub.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_full_preset_json_roundtrip() {
        let mut preset = FullPreset::default();
        preset.title = "Video Clean".to_string();
        preset.suffix = "_{res}_{vcodec}".to_string();
        preset.sync_sidecars = true;
        preset.organize_enabled = true;
        preset.organize_template = "{year}/{month}".to_string();
        preset.extra_replaces.push(ReplaceStep {
            find: " ".to_string(),
            replace: "_".to_string(),
            use_regex: false,
            case_sensitive: true,
        });

        let json = serde_json::to_string(&preset).unwrap();
        let back: FullPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, "Video Clean");
        assert_eq!(back.organize_template, "{year}/{month}");
        assert_eq!(back.extra_replaces.len(), 1);
        assert!(back.sync_sidecars);
    }
}
