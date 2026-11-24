use crate::rename_engine::{
    compute_preview, execute_rename_batch, load_presets_json, FullPreset, HashCache, Lang,
    RenameConfig, RowIssue,
};
use crate::scanner;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

pub enum CliAction {
    LaunchGui(Option<PathBuf>),
    ExitCode(i32),
}

#[cfg(windows)]
fn attach_parent_console() {
    extern "system" {
        fn AttachConsole(dw_process_id: u32) -> i32;
    }
    unsafe {
        let _ = AttachConsole(0xFFFF_FFFF);
    }
}

fn print_help(app_version: &str) {
    println!("ReStem v{} — Fast Bulk File & Folder Renamer", app_version);
    println!();
    println!("USAGE:");
    println!("  restem [DIR]                                 Open GUI (optional folder)");
    println!("  restem <DIR> --preset <NAME> [OPTIONS]       Run saved preset in CLI mode");
    println!("  restem <DIR> --preset-file <FILE> [OPTIONS]  Run JSON preset file in CLI mode");
    println!("  restem --list-presets                        Show saved user presets");
    println!("  restem --version | --help                    Print version or help");
    println!();
    println!("OPTIONS:");
    println!("  --recursive    Include subdirectories");
    println!("  --dry-run      Preview changes only (default when --apply is omitted)");
    println!("  --apply        Execute renames on disk");
}

pub fn handle_cli_args(args: &[String], app_version: &str) -> CliAction {
    let has_flag = args.iter().skip(1).any(|a| a.starts_with('-'));
    if !has_flag {
        let cli_dir = args.get(1).map(PathBuf::from).and_then(|p| {
            if p.is_dir() {
                Some(p)
            } else {
                p.parent().map(|par| par.to_path_buf())
            }
        });
        return CliAction::LaunchGui(cli_dir);
    }

    #[cfg(windows)]
    attach_parent_console();

    let mut dir: Option<PathBuf> = None;
    let mut preset_name: Option<String> = None;
    let mut preset_file: Option<PathBuf> = None;
    let mut recursive = false;
    let mut apply = false;
    let mut dry_run = false;

    let mut i = 1usize;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-h" | "--help" => {
                print_help(app_version);
                return CliAction::ExitCode(0);
            }
            "-V" | "--version" => {
                println!("ReStem v{}", app_version);
                return CliAction::ExitCode(0);
            }
            "--list-presets" => {
                let presets = load_presets_json();
                if presets.is_empty() {
                    println!("No saved presets found.");
                } else {
                    for p in presets {
                        println!("* {}", p.title);
                    }
                }
                return CliAction::ExitCode(0);
            }
            "--recursive" | "-r" => recursive = true,
            "--apply" => apply = true,
            "--dry-run" => dry_run = true,
            "--preset" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    preset_name = Some(v.clone());
                } else {
                    eprintln!("Error: --preset requires a preset name");
                    return CliAction::ExitCode(1);
                }
            }
            "--preset-file" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    preset_file = Some(PathBuf::from(v));
                } else {
                    eprintln!("Error: --preset-file requires a JSON path");
                    return CliAction::ExitCode(1);
                }
            }
            other if !other.starts_with('-') => {
                if dir.is_none() {
                    dir = Some(PathBuf::from(other));
                }
            }
            unknown => {
                eprintln!("Unknown option: {}", unknown);
                return CliAction::ExitCode(1);
            }
        }
        i += 1;
    }

    let Some(target_dir) = dir else {
        eprintln!("Error: target directory is required. Run --help for usage.");
        return CliAction::ExitCode(1);
    };
    if !target_dir.is_dir() {
        eprintln!("Error: directory does not exist: {}", target_dir.display());
        return CliAction::ExitCode(1);
    }

    let preset = if let Some(pf) = preset_file {
        let Ok(raw) = fs::read_to_string(&pf) else {
            eprintln!("Error: cannot read preset file: {}", pf.display());
            return CliAction::ExitCode(1);
        };
        if let Ok(single) = serde_json::from_str::<FullPreset>(&raw) {
            single
        } else if let Ok(list) = serde_json::from_str::<Vec<FullPreset>>(&raw) {
            if let Some(name) = &preset_name {
                let Some(found) = list.into_iter().find(|p| p.title.eq_ignore_ascii_case(name))
                else {
                    eprintln!("Error: preset '{}' not found in {}", name, pf.display());
                    return CliAction::ExitCode(1);
                };
                found
            } else if let Some(first) = list.into_iter().next() {
                first
            } else {
                eprintln!("Error: preset file is empty: {}", pf.display());
                return CliAction::ExitCode(1);
            }
        } else {
            eprintln!("Error: invalid preset JSON in {}", pf.display());
            return CliAction::ExitCode(1);
        }
    } else if let Some(name) = preset_name {
        let presets = load_presets_json();
        let Some(found) = presets
            .into_iter()
            .find(|p| p.title.eq_ignore_ascii_case(&name))
        else {
            eprintln!("Error: saved preset '{}' not found.", name);
            return CliAction::ExitCode(1);
        };
        found
    } else {
        eprintln!("Error: specify --preset <NAME> or --preset-file <FILE>.");
        return CliAction::ExitCode(1);
    };

    let cfg = RenameConfig::from(&preset);
    let (entries, _) = scanner::scan_directory_sync(&target_dir, recursive, true, true);
    let excl = HashSet::new();
    let ov = HashMap::new();
    let mut hash_cache = HashCache::new();
    let summary = compute_preview(Some(&target_dir), &entries, &excl, &ov, &cfg, &mut hash_cache);

    println!(
        "Scanned: {} | Ready to rename: {} | Issues: {}",
        summary.total_filtered, summary.changed_count, summary.issue_count
    );

    for row in &summary.rows {
        if row.changed || row.issue != RowIssue::None {
            let dest_disp = if row.target_rel_dir != "." && row.target_rel_dir != row.rel_parent {
                format!("{}/{}", row.target_rel_dir, row.new_name)
            } else {
                row.new_name.clone()
            };
            if row.issue != RowIssue::None {
                println!(
                    "  [ISSUE: {}] {} -> {}",
                    row.issue.label(Lang::En),
                    row.old_name,
                    dest_disp
                );
            } else {
                println!("  {} -> {}", row.old_name, dest_disp);
            }
        }
    }

    if summary.issue_count > 0 {
        eprintln!("Aborted: resolve {} collision/name issue(s) first.", summary.issue_count);
        return CliAction::ExitCode(1);
    }

    if apply && !dry_run {
        match execute_rename_batch(&summary.rows, Lang::En) {
            Ok(batch) => {
                println!("Successfully renamed {} item(s).", batch.moves.len());
                CliAction::ExitCode(0)
            }
            Err(e) => {
                eprintln!("Rename failed: {}", e);
                CliAction::ExitCode(1)
            }
        }
    } else {
        if summary.changed_count > 0 {
            println!("Dry-run complete. Re-run with --apply to rename files on disk.");
        }
        CliAction::ExitCode(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_flags_and_dry_run() {
        let gui = handle_cli_args(&["restem.exe".to_string()], "1.5.0");
        assert!(matches!(gui, CliAction::LaunchGui(None)));

        let ver = handle_cli_args(&["restem.exe".to_string(), "--version".to_string()], "1.5.0");
        assert!(matches!(ver, CliAction::ExitCode(0)));

        let dir = std::env::temp_dir().join(format!("restem_cli_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("old name.txt");
        fs::write(&file, b"hi").unwrap();

        let mut p = FullPreset::default();
        p.title = "Underscore".to_string();
        p.find_str = " ".to_string();
        p.replace_str = "_".to_string();
        let pfile = dir.join("preset.json");
        fs::write(&pfile, serde_json::to_string(&p).unwrap()).unwrap();

        let args = vec![
            "restem.exe".to_string(),
            dir.to_string_lossy().to_string(),
            "--preset-file".to_string(),
            pfile.to_string_lossy().to_string(),
            "--dry-run".to_string(),
        ];
        let res = handle_cli_args(&args, "1.5.0");
        assert!(matches!(res, CliAction::ExitCode(0)));
        assert!(file.exists()); // Dry-run left file intact

        let _ = fs::remove_dir_all(&dir);
    }
}
