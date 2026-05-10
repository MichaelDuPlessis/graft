use crate::config::{LinkMode, expand_tilde};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

pub fn deploy_files(
    files: &HashMap<String, String>,
    config_dir: &Path,
    link_mode: LinkMode,
    force: bool,
    dry_run: bool,
) -> Vec<String> {
    let mut errors = Vec::new();

    for (source, dest) in files {
        let src_path = config_dir.join(source);
        if !src_path.exists() {
            errors.push(format!("Source not found: {}", src_path.display()));
            continue;
        }

        let dest_path = expand_tilde(dest);

        if dry_run {
            let action = match link_mode {
                LinkMode::Symlink => "symlink",
                LinkMode::Copy => "copy",
            };
            println!(
                "  {} {} → {}",
                action,
                src_path.display(),
                dest_path.display()
            );
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                errors.push(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ));
                continue;
            }
        }

        let src_canonical = match fs::canonicalize(&src_path) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("Failed to resolve {}: {}", src_path.display(), e));
                continue;
            }
        };

        if is_our_symlink(&dest_path, &src_canonical) {
            continue;
        }

        if dest_path.symlink_metadata().is_ok() {
            if !force {
                errors.push(format!(
                    "Destination already exists (use --force to overwrite): {}",
                    dest_path.display()
                ));
                continue;
            }
            if dest_path.is_dir() && !dest_path.is_symlink() {
                if let Err(e) = fs::remove_dir_all(&dest_path) {
                    errors.push(format!("Failed to remove {}: {}", dest_path.display(), e));
                    continue;
                }
            } else if let Err(e) = fs::remove_file(&dest_path) {
                errors.push(format!("Failed to remove {}: {}", dest_path.display(), e));
                continue;
            }
        }

        let result = match link_mode {
            LinkMode::Symlink => symlink(&src_canonical, &dest_path),
            LinkMode::Copy => {
                if src_path.is_dir() {
                    copy_dir_recursive(&src_path, &dest_path)
                } else {
                    fs::copy(&src_path, &dest_path).map(|_| ())
                }
            }
        };

        if let Err(e) = result {
            errors.push(format!(
                "Failed to deploy {} → {}: {}",
                src_path.display(),
                dest_path.display(),
                e
            ));
        }
    }

    errors
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest_entry = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_entry)?;
        } else {
            fs::copy(entry.path(), &dest_entry)?;
        }
    }
    Ok(())
}

fn is_our_symlink(dest: &Path, source: &Path) -> bool {
    match fs::read_link(dest) {
        Ok(target) => {
            // Compare canonicalized paths to handle relative symlinks
            match fs::canonicalize(&target) {
                Ok(canonical_target) => canonical_target == source,
                Err(_) => target == source,
            }
        }
        Err(_) => false,
    }
}
