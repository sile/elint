//! Collects Erlang source paths under a file or directory.

/// Walks `path` and returns every `.erl` / `.hrl` file.
///
/// A file path is returned as a single-element vec when its extension matches.
/// A directory is scanned recursively.
pub fn collect_erlang_files<P: AsRef<std::path::Path>>(
    path: P,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let path = path.as_ref();

    if path.is_file() {
        if is_erlang_file(path) {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        collect_erlang_files_recursive(path, &mut files)?;
    }

    Ok(files)
}

fn collect_erlang_files_recursive(
    path: &std::path::Path,
    files: &mut Vec<std::path::PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_erlang_files_recursive(&path, files)?;
        } else if is_erlang_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_erlang_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("erl" | "hrl")
    )
}
