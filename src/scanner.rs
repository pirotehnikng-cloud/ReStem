use crate::audio_info;
use crate::media_info;
use crate::photo_meta;
use crate::rename_engine::{natural_cmp, FsEntry};
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::UNIX_EPOCH;

pub enum ScanMsg {
    Progress {
        scanned: usize,
        current_name: String,
    },
    Done {
        root_dir: PathBuf,
        entries: Vec<FsEntry>,
        subfolders: Vec<PathBuf>,
        cancelled: bool,
    },
}

pub struct ScanJob {
    pub rx: mpsc::Receiver<ScanMsg>,
    pub cancel: Arc<AtomicBool>,
    pub scanned: usize,
    pub current_name: String,
}

pub fn is_entry_hidden(path: &Path, meta: Option<&fs::Metadata>) -> bool {
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

fn collect_subfolders(root_dir: &Path, hide_system_hidden: bool) -> Vec<PathBuf> {
    let mut subs = Vec::new();
    if let Ok(read_top) = fs::read_dir(root_dir) {
        for e in read_top.flatten() {
            let p = e.path();
            if p.is_dir() {
                let meta = e.metadata().ok();
                if hide_system_hidden && is_entry_hidden(&p, meta.as_ref()) {
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
    subs
}

fn collect_recursive(
    root: &Path,
    current: &Path,
    depth: usize,
    recursive: bool,
    hide_system_hidden: bool,
    use_external_mediainfo: bool,
    cancel: Option<&AtomicBool>,
    out: &mut Vec<FsEntry>,
    on_progress: &mut dyn FnMut(usize, &str),
) -> bool {
    let Ok(read_dir) = fs::read_dir(current) else {
        return false;
    };

    for entry in read_dir.flatten() {
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return true;
            }
        }

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

        let is_hidden = is_entry_hidden(&path, meta.as_ref());
        if hide_system_hidden && is_hidden {
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

        let is_image = !is_dir && photo_meta::is_photo_meta_ext(&ext_lower);
        let is_video = !is_dir && media_info::is_video_ext(&ext_lower);
        let is_audio = !is_dir && audio_info::is_audio_ext(&ext_lower);

        let (photo_meta, video_info, audio_info) = if is_image {
            (Some(photo_meta::extract_photo_meta(&path, &ext_lower)), None, None)
        } else if is_video {
            (
                None,
                media_info::extract_video_info(&path, size, use_external_mediainfo),
                None,
            )
        } else if is_audio {
            (None, None, audio_info::extract_audio_info(&path, size))
        } else {
            (None, None, None)
        };

        let cur_name_ref = name.clone();
        out.push(FsEntry {
            path: path.clone(),
            parent_dir,
            parent_name,
            rel_parent: rel_display,
            name,
            ext_lower,
            is_dir,
            is_image,
            is_video,
            is_audio,
            is_hidden,
            depth,
            size,
            modified_ts,
            photo_meta,
            video_info,
            audio_info,
        });

        on_progress(out.len(), &cur_name_ref);

        if is_dir && recursive {
            let cancelled = collect_recursive(
                root,
                &path,
                depth + 1,
                recursive,
                hide_system_hidden,
                use_external_mediainfo,
                cancel,
                out,
                on_progress,
            );
            if cancelled {
                return true;
            }
        }
    }
    false
}

pub fn scan_directory_sync(
    root_dir: &Path,
    recursive: bool,
    hide_system_hidden: bool,
    use_external_mediainfo: bool,
) -> (Vec<FsEntry>, Vec<PathBuf>) {
    let subs = collect_subfolders(root_dir, hide_system_hidden);
    let mut entries = Vec::new();
    let mut noop = |_count: usize, _name: &str| {};
    collect_recursive(
        root_dir,
        root_dir,
        0,
        recursive,
        hide_system_hidden,
        use_external_mediainfo,
        None,
        &mut entries,
        &mut noop,
    );
    (entries, subs)
}

pub fn start_background_scan(
    root_dir: PathBuf,
    recursive: bool,
    hide_system_hidden: bool,
    use_external_mediainfo: bool,
    ctx: egui::Context,
) -> ScanJob {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_worker = Arc::clone(&cancel);

    std::thread::spawn(move || {
        let subs = collect_subfolders(&root_dir, hide_system_hidden);
        let mut entries = Vec::new();
        let mut progress_cb = |count: usize, name: &str| {
            if count <= 10 || count % 8 == 0 {
                let _ = tx.send(ScanMsg::Progress {
                    scanned: count,
                    current_name: name.to_string(),
                });
                ctx.request_repaint();
            }
        };

        let cancelled = collect_recursive(
            &root_dir,
            &root_dir,
            0,
            recursive,
            hide_system_hidden,
            use_external_mediainfo,
            Some(&cancel_worker),
            &mut entries,
            &mut progress_cb,
        );

        let _ = tx.send(ScanMsg::Done {
            root_dir,
            entries,
            subfolders: subs,
            cancelled,
        });
        ctx.request_repaint();
    });

    ScanJob {
        rx,
        cancel,
        scanned: 0,
        current_name: String::new(),
    }
}
