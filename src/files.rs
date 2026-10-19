use std::path::{Path, PathBuf};
use url::Url;

fn local_path(file: &str) -> Result<String, String> {
  let path = if Path::new(file).is_absolute() {
    PathBuf::from(file)
  } else {
    if file.chars().any(char::is_control) {
      return Err("Control characters in file URIs must be percent-encoded".into());
    }
    let uri =
      Url::parse(file).map_err(|_| "Files must be absolute paths or file:// URIs".to_string())?;
    if uri.scheme() != "file" || uri.query().is_some() || uri.fragment().is_some() {
      return Err(
        "Files must be absolute paths or file:// URIs without queries or fragments".into(),
      );
    }
    uri
      .to_file_path()
      .map_err(|_| "File URI cannot be represented as a local path on this platform".to_string())?
  };

  let path = path
    .into_os_string()
    .into_string()
    .map_err(|_| "File paths must be valid UTF-8".to_string())?;
  if path.contains('\0') {
    return Err("File paths must be valid UTF-8 without NUL bytes".into());
  }
  Ok(path)
}

pub(crate) fn local_paths(files: Vec<String>) -> Result<Vec<String>, String> {
  files.iter().map(|file| local_path(file)).collect()
}

pub(crate) fn clipboard_files(files: Vec<String>) -> Result<Vec<String>, String> {
  let paths = local_paths(files)?;
  // X11 and Wayland exchange URI lists; Windows and macOS backends expect native paths.
  #[cfg(target_os = "linux")]
  {
    paths
      .iter()
      .map(|path| {
        Url::from_file_path(path)
          .map(String::from)
          .map_err(|_| "Failed to encode file path as a file URI".to_string())
      })
      .collect()
  }
  #[cfg(not(target_os = "linux"))]
  {
    Ok(paths)
  }
}
