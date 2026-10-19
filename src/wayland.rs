use crate::{ClipboardContents, ClipboardData, ImageData, ListenerState, files};
use clipboard_rs::common::{RustImage, RustImageData};
use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use wl_clipboard_rs::copy::{
  self, ClipboardType as CopyClipboardType, Error as CopyError, MimeSource as CopyMimeSource,
  MimeType as CopyMimeType, Options as CopyOptions, Seat as CopySeat, Source as CopySource,
};
use wl_clipboard_rs::paste::{
  self, ClipboardType as PasteClipboardType, Error as PasteError, MimeType as PasteMimeType,
  Seat as PasteSeat,
};

use wl_clipboard_rs::watch::{CancelHandle, ClipboardEvent, ClipboardType, Watcher};

macro_rules! wayland_log {
  ($($arg:tt)*) => {{
    if crate::is_debug_logging_enabled() {
      eprintln!(
        "[clipboard-rs][listener][{:?}] {}",
        std::thread::current().id(),
        format!($($arg)*)
      );
    }
  }};
}

pub(crate) fn is_wayland_environment() -> bool {
  if std::env::var("WAYLAND_DISPLAY").is_ok_and(|display| !display.is_empty()) {
    return true;
  }

  if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE")
    && session_type == "wayland"
  {
    return true;
  }

  false
}

pub(crate) fn is_wayland_clipboard_available() -> bool {
  if !is_wayland_environment() {
    wayland_log!(
      "is_wayland_clipboard_available=false: not in wayland env (WAYLAND_DISPLAY={:?}, XDG_SESSION_TYPE={:?})",
      std::env::var("WAYLAND_DISPLAY").ok(),
      std::env::var("XDG_SESSION_TYPE").ok()
    );
    return false;
  }

  match Watcher::new(ClipboardType::Regular, PasteSeat::Unspecified) {
    Ok(_) => {
      wayland_log!("is_wayland_clipboard_available=true");
      true
    }
    Err(e) => {
      wayland_log!("is_wayland_clipboard_available=false: init failed: {}", e);
      false
    }
  }
}

fn is_wayland_image_mime(mime_type: &str) -> bool {
  let normalized = mime_type.trim().to_ascii_lowercase();
  normalized.starts_with("image/") || normalized == "application/x-qt-image"
}

fn is_wayland_text_mime(mime_type: &str) -> bool {
  matches!(
    mime_type.trim().to_ascii_lowercase().as_str(),
    "text/plain" | "text/plain;charset=utf-8" | "utf8_string" | "text" | "string"
  )
}

fn is_wayland_html_mime(mime_type: &str) -> bool {
  mime_type.trim().eq_ignore_ascii_case("text/html")
}

fn is_wayland_rtf_mime(mime_type: &str) -> bool {
  matches!(
    mime_type.trim().to_ascii_lowercase().as_str(),
    "text/rtf" | "text/richtext" | "application/rtf" | "application/x-rtf"
  )
}

fn is_wayland_files_mime(mime_type: &str) -> bool {
  matches!(
    mime_type.trim().to_ascii_lowercase().as_str(),
    "text/uri-list" | "x-special/gnome-copied-files" | "x-special/nautilus-clipboard"
  )
}

fn has_wayland_format(formats: &[String], name: &str) -> bool {
  formats.iter().any(|format| format == name)
}

fn push_wayland_format(formats: &mut Vec<String>, name: &str) {
  if !has_wayland_format(formats, name) {
    formats.push(name.to_string());
  }
}

fn extend_wayland_formats_with_custom_mimes(formats: &mut Vec<String>, offered_mimes: &[String]) {
  for mime in offered_mimes {
    let is_well_known_standard = is_wayland_text_mime(mime)
      || is_wayland_rtf_mime(mime)
      || is_wayland_html_mime(mime)
      || is_wayland_image_mime(mime)
      || is_wayland_files_mime(mime);
    if !is_well_known_standard {
      push_wayland_format(formats, mime);
    }
  }
}

fn infer_wayland_available_formats(offered_mimes: &[String]) -> Vec<String> {
  let mut has_text = false;
  let mut has_rtf = false;
  let mut has_html = false;
  let mut has_image = false;
  let mut has_files = false;

  for mime_type in offered_mimes {
    if is_wayland_text_mime(mime_type) {
      has_text = true;
    }
    if is_wayland_rtf_mime(mime_type) {
      has_rtf = true;
    }
    if is_wayland_html_mime(mime_type) {
      has_html = true;
    }
    if is_wayland_image_mime(mime_type) {
      has_image = true;
    }
    if is_wayland_files_mime(mime_type) {
      has_files = true;
    }
  }

  let mut formats = Vec::new();
  if has_text {
    formats.push("text".to_string());
  }
  if has_rtf {
    formats.push("rtf".to_string());
  }
  if has_html {
    formats.push("html".to_string());
  }
  if has_image {
    formats.push("image".to_string());
  }
  if has_files {
    formats.push("files".to_string());
  }
  formats
}

fn decode_wayland_files(payload: &[u8]) -> WaylandResult<Vec<String>> {
  let file_list =
    std::str::from_utf8(payload).map_err(|e| format!("Invalid clipboard files encoding: {e}"))?;
  let mut files = Vec::new();

  for line in file_list.lines() {
    let item = line;
    if item.is_empty() || item.starts_with('#') {
      continue;
    }

    if item.eq_ignore_ascii_case("copy") || item.eq_ignore_ascii_case("cut") {
      continue;
    }

    files.push(item.to_string());
  }

  files::local_paths(files)
}

fn detect_wayland_image_magic(payload: &[u8]) -> Option<&'static str> {
  if payload.starts_with(b"\x89PNG\r\n\x1a\n") {
    return Some("png");
  }
  if payload.starts_with(b"\xff\xd8\xff") {
    return Some("jpeg");
  }
  if payload.starts_with(b"GIF87a") || payload.starts_with(b"GIF89a") {
    return Some("gif");
  }
  if payload.starts_with(b"BM") {
    return Some("bmp");
  }
  if payload.len() >= 12 && &payload[0..4] == b"RIFF" && &payload[8..12] == b"WEBP" {
    return Some("webp");
  }
  None
}

const WAYLAND_RTF_MIME_PRIORITY: &[&str] = &[
  "text/rtf",
  "application/rtf",
  "application/x-rtf",
  "text/richtext",
];

const WAYLAND_TEXT_MIME_PRIORITY: &[&str] = &[
  "text/plain;charset=utf-8",
  "text/plain",
  "utf8_string",
  "text",
  "string",
];

const WAYLAND_HTML_MIME_PRIORITY: &[&str] = &["text/html"];

const WAYLAND_IMAGE_MIME_PRIORITY: &[&str] = &[
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/bmp",
  "image/gif",
  "application/x-qt-image",
];

const WAYLAND_FILES_MIME_PRIORITY: &[&str] = &[
  "text/uri-list",
  "x-special/gnome-copied-files",
  "x-special/nautilus-clipboard",
];

type WaylandResult<T> = std::result::Result<T, String>;
type ContentReader<'a> = dyn FnMut(&str) -> WaylandResult<Vec<u8>> + 'a;

fn wayland_paste_error_detail(err: &PasteError) -> String {
  err.to_string()
}

fn wayland_copy_error_detail(err: &CopyError) -> String {
  err.to_string()
}

fn get_wayland_mime_types_ordered() -> WaylandResult<Vec<String>> {
  paste::get_mime_types_ordered(PasteClipboardType::Regular, PasteSeat::Unspecified).map_err(|e| {
    format!(
      "Failed to query MIME types: {}",
      wayland_paste_error_detail(&e)
    )
  })
}

fn get_wayland_mime_types_ordered_or_empty() -> WaylandResult<Vec<String>> {
  match paste::get_mime_types_ordered(PasteClipboardType::Regular, PasteSeat::Unspecified) {
    Ok(mimes) => Ok(mimes),
    Err(PasteError::ClipboardEmpty) | Err(PasteError::NoSeats) => Ok(Vec::new()),
    Err(e) => Err(format!(
      "Failed to query MIME types: {}",
      wayland_paste_error_detail(&e)
    )),
  }
}

fn get_wayland_contents_bytes(
  requested_mime: PasteMimeType<'_>,
) -> WaylandResult<(Vec<u8>, String)> {
  let (mut pipe, actual_mime) = paste::get_contents(
    PasteClipboardType::Regular,
    PasteSeat::Unspecified,
    requested_mime,
  )
  .map_err(|e| {
    format!(
      "Failed to read clipboard content: {}",
      wayland_paste_error_detail(&e)
    )
  })?;

  let mut payload = Vec::new();
  pipe
    .read_to_end(&mut payload)
    .map_err(|e| format!("Failed to read clipboard stream: {e}"))?;

  Ok((payload, actual_mime))
}

fn find_wayland_mime<'a>(offered_mimes: &'a [String], preferred: &[&str]) -> Option<&'a str> {
  for desired in preferred {
    if let Some(found) = offered_mimes
      .iter()
      .find(|mime| mime.eq_ignore_ascii_case(desired))
    {
      return Some(found.as_str());
    }
  }
  None
}

fn decode_utf8_payload_lossy(payload: Vec<u8>, format_name: &str, mime: &str) -> String {
  match String::from_utf8(payload) {
    Ok(text) => text,
    Err(e) => {
      wayland_log!(
        "Clipboard {} is not valid UTF-8, using lossy decoding: mime={}",
        format_name,
        mime
      );
      String::from_utf8_lossy(&e.into_bytes()).into_owned()
    }
  }
}

fn read_wayland_textual_content(
  offered_mimes: &[String],
  preferred: &[&str],
  fallback_predicate: fn(&str) -> bool,
  format_name: &str,
  read: &mut ContentReader<'_>,
) -> Option<String> {
  let selected_mime = find_wayland_mime(offered_mimes, preferred).or_else(|| {
    offered_mimes
      .iter()
      .find(|mime| fallback_predicate(mime.as_str()))
      .map(|mime| mime.as_str())
  })?;
  let payload = read(selected_mime).map_err(|e| wayland_log!("{e}")).ok()?;
  Some(decode_utf8_payload_lossy(
    payload,
    format_name,
    selected_mime,
  ))
}

fn read_wayland_image_content(
  offered_mimes: &[String],
  read: &mut ContentReader<'_>,
) -> Option<ImageData> {
  let selected_mime =
    find_wayland_mime(offered_mimes, WAYLAND_IMAGE_MIME_PRIORITY).or_else(|| {
      offered_mimes
        .iter()
        .find(|mime| is_wayland_image_mime(mime))
        .map(|mime| mime.as_str())
    })?;
  let payload = read(selected_mime).map_err(|e| wayland_log!("{e}")).ok()?;
  to_wayland_image_data(payload)
    .map_err(|e| wayland_log!("{e}"))
    .ok()
}

fn read_wayland_files_content(
  offered_mimes: &[String],
  read: &mut ContentReader<'_>,
) -> Option<Vec<String>> {
  let selected_mime =
    find_wayland_mime(offered_mimes, WAYLAND_FILES_MIME_PRIORITY).or_else(|| {
      offered_mimes
        .iter()
        .find(|mime| is_wayland_files_mime(mime))
        .map(|mime| mime.as_str())
    })?;
  let payload = read(selected_mime).map_err(|e| wayland_log!("{e}")).ok()?;
  decode_wayland_files(&payload)
    .map_err(|e| wayland_log!("{e}"))
    .ok()
}

fn read_wayland_complete_data_from_mimes(
  offered_mimes: &[String],
  read: &mut ContentReader<'_>,
) -> ClipboardData {
  let mut available_formats = infer_wayland_available_formats(offered_mimes);
  extend_wayland_formats_with_custom_mimes(&mut available_formats, offered_mimes);

  let text = if has_wayland_format(&available_formats, "text") {
    read_wayland_textual_content(
      offered_mimes,
      WAYLAND_TEXT_MIME_PRIORITY,
      is_wayland_text_mime,
      "text",
      read,
    )
  } else {
    None
  };

  let html = if has_wayland_format(&available_formats, "html") {
    read_wayland_textual_content(
      offered_mimes,
      WAYLAND_HTML_MIME_PRIORITY,
      is_wayland_html_mime,
      "html",
      read,
    )
  } else {
    None
  };

  let rtf = if has_wayland_format(&available_formats, "rtf") {
    read_wayland_textual_content(
      offered_mimes,
      WAYLAND_RTF_MIME_PRIORITY,
      is_wayland_rtf_mime,
      "rich text",
      read,
    )
  } else {
    None
  };

  let image = if has_wayland_format(&available_formats, "image") {
    read_wayland_image_content(offered_mimes, read)
  } else {
    None
  };

  let files = if has_wayland_format(&available_formats, "files") {
    read_wayland_files_content(offered_mimes, read)
  } else {
    None
  };

  ClipboardData {
    available_formats,
    text,
    rtf,
    html,
    image,
    files,
  }
}

fn to_wayland_image_data(payload: Vec<u8>) -> WaylandResult<ImageData> {
  let image_data = RustImageData::from_bytes(&payload)
    .map_err(|e| format!("Failed to decode clipboard image: {e}"))?;
  let (width, height) = image_data.get_size();
  let png_data = image_data
    .to_png()
    .map_err(|e| format!("Failed to convert clipboard image to PNG: {e}"))?;
  let bytes = png_data.get_bytes();
  Ok(ImageData {
    width,
    height,
    size: bytes.len() as u32,
    data: Buffer::from(bytes.to_vec()),
  })
}

fn append_wayland_file_sources(target: &mut Vec<CopyMimeSource>, files: &[String]) {
  if files.is_empty() {
    return;
  }

  let mut uri_list_payload = files.join("\r\n");
  uri_list_payload.push_str("\r\n");

  target.push(CopyMimeSource {
    source: CopySource::Bytes(uri_list_payload.into_bytes().into_boxed_slice()),
    mime_type: CopyMimeType::Specific("text/uri-list".to_string()),
  });

  let mut gnome_payload = String::from("copy\n");
  gnome_payload.push_str(&files.join("\n"));
  gnome_payload.push('\n');

  target.push(CopyMimeSource {
    source: CopySource::Bytes(gnome_payload.as_bytes().to_vec().into_boxed_slice()),
    mime_type: CopyMimeType::Specific("x-special/gnome-copied-files".to_string()),
  });

  target.push(CopyMimeSource {
    source: CopySource::Bytes(gnome_payload.into_bytes().into_boxed_slice()),
    mime_type: CopyMimeType::Specific("x-special/nautilus-clipboard".to_string()),
  });
}

fn append_wayland_rtf_sources(target: &mut Vec<CopyMimeSource>, rtf: &str) {
  for mime in WAYLAND_RTF_MIME_PRIORITY {
    target.push(CopyMimeSource {
      source: CopySource::Bytes(rtf.as_bytes().to_vec().into_boxed_slice()),
      mime_type: CopyMimeType::Specific((*mime).to_string()),
    });
  }
}

fn wayland_copy_single(source: Vec<u8>, mime_type: CopyMimeType) -> WaylandResult<()> {
  let options = CopyOptions::new();
  options
    .copy(CopySource::Bytes(source.into_boxed_slice()), mime_type)
    .map_err(|e| {
      format!(
        "Failed to write clipboard content: {}",
        wayland_copy_error_detail(&e)
      )
    })
}

fn wayland_copy_multi(sources: Vec<CopyMimeSource>) -> WaylandResult<()> {
  let options = CopyOptions::new();
  options.copy_multi(sources).map_err(|e| {
    format!(
      "Failed to write clipboard content (multi mime): {}",
      wayland_copy_error_detail(&e)
    )
  })
}

pub(crate) fn get_text() -> WaylandResult<String> {
  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Text)?;
  String::from_utf8(payload).map_err(|e| format!("Clipboard text is not valid UTF-8: {e}"))
}

pub(crate) fn set_text(text: String) -> WaylandResult<()> {
  wayland_copy_single(text.into_bytes(), CopyMimeType::Text)
}

pub(crate) fn get_html() -> WaylandResult<String> {
  let offered_mimes = get_wayland_mime_types_ordered()?;
  let selected_mime = find_wayland_mime(&offered_mimes, &["text/html"])
    .ok_or_else(|| "Clipboard does not contain HTML data".to_string())?;
  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Specific(selected_mime))?;
  String::from_utf8(payload).map_err(|e| format!("Clipboard HTML is not valid UTF-8: {e}"))
}

pub(crate) fn set_html(html: String) -> WaylandResult<()> {
  wayland_copy_single(
    html.into_bytes(),
    CopyMimeType::Specific("text/html".to_string()),
  )
}

pub(crate) fn get_rich_text() -> WaylandResult<String> {
  let offered_mimes = get_wayland_mime_types_ordered()?;
  let selected_mime = find_wayland_mime(&offered_mimes, WAYLAND_RTF_MIME_PRIORITY)
    .or_else(|| {
      offered_mimes
        .iter()
        .find(|mime| is_wayland_rtf_mime(mime))
        .map(|x| x.as_str())
    })
    .ok_or_else(|| "Clipboard does not contain rich text data".to_string())?;
  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Specific(selected_mime))?;
  Ok(decode_utf8_payload_lossy(
    payload,
    "rich text",
    selected_mime,
  ))
}

pub(crate) fn set_rich_text(text: String) -> WaylandResult<()> {
  let mut sources = Vec::new();
  append_wayland_rtf_sources(&mut sources, &text);
  wayland_copy_multi(sources)
}

pub(crate) fn get_image_raw() -> WaylandResult<Vec<u8>> {
  let offered_mimes = get_wayland_mime_types_ordered()?;
  let selected_mime = find_wayland_mime(&offered_mimes, WAYLAND_IMAGE_MIME_PRIORITY)
    .or_else(|| {
      offered_mimes
        .iter()
        .find(|mime| is_wayland_image_mime(mime))
        .map(|x| x.as_str())
    })
    .ok_or_else(|| "Clipboard does not contain image data".to_string())?;

  let (payload, actual_mime) = get_wayland_contents_bytes(PasteMimeType::Specific(selected_mime))?;
  wayland_log!(
    "get_image_raw: selected_mime={}, actual_mime={}, bytes={}",
    selected_mime,
    actual_mime,
    payload.len()
  );
  Ok(payload)
}

pub(crate) fn set_image_raw(image_data: Vec<u8>) -> WaylandResult<()> {
  let mime_type = match detect_wayland_image_magic(&image_data) {
    Some("png") => CopyMimeType::Specific("image/png".to_string()),
    Some("jpeg") => CopyMimeType::Specific("image/jpeg".to_string()),
    Some("gif") => CopyMimeType::Specific("image/gif".to_string()),
    Some("bmp") => CopyMimeType::Specific("image/bmp".to_string()),
    Some("webp") => CopyMimeType::Specific("image/webp".to_string()),
    _ => CopyMimeType::Autodetect,
  };

  wayland_copy_single(image_data, mime_type)
}

pub(crate) fn get_files() -> WaylandResult<Vec<String>> {
  let offered_mimes = get_wayland_mime_types_ordered()?;
  let selected_mime = find_wayland_mime(&offered_mimes, WAYLAND_FILES_MIME_PRIORITY)
    .or_else(|| {
      offered_mimes
        .iter()
        .find(|mime| is_wayland_files_mime(mime))
        .map(|x| x.as_str())
    })
    .ok_or_else(|| "Clipboard does not contain files data".to_string())?;

  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Specific(selected_mime))?;
  decode_wayland_files(&payload)
}

pub(crate) fn set_files(files: Vec<String>) -> WaylandResult<()> {
  if files.is_empty() {
    return clear();
  }

  let mut sources = Vec::new();
  append_wayland_file_sources(&mut sources, &files);
  wayland_copy_multi(sources)
}

pub(crate) fn set_buffer(format: String, buffer: Vec<u8>) -> WaylandResult<()> {
  wayland_copy_single(buffer, CopyMimeType::Specific(format))
}

pub(crate) fn get_buffer(format: String) -> WaylandResult<Vec<u8>> {
  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Specific(&format))?;
  Ok(payload)
}

pub(crate) fn set_contents(contents: ClipboardContents) -> WaylandResult<()> {
  let mut sources = Vec::new();

  if let Some(text) = contents.text {
    sources.push(CopyMimeSource {
      source: CopySource::Bytes(text.into_bytes().into_boxed_slice()),
      mime_type: CopyMimeType::Text,
    });
  }

  if let Some(html) = contents.html {
    sources.push(CopyMimeSource {
      source: CopySource::Bytes(html.into_bytes().into_boxed_slice()),
      mime_type: CopyMimeType::Specific("text/html".to_string()),
    });
  }

  if let Some(rtf) = contents.rtf {
    append_wayland_rtf_sources(&mut sources, &rtf);
  }

  if let Some(image_data) = contents.image {
    let mime_type = match detect_wayland_image_magic(image_data.data.as_ref()) {
      Some("png") => CopyMimeType::Specific("image/png".to_string()),
      Some("jpeg") => CopyMimeType::Specific("image/jpeg".to_string()),
      Some("gif") => CopyMimeType::Specific("image/gif".to_string()),
      Some("bmp") => CopyMimeType::Specific("image/bmp".to_string()),
      Some("webp") => CopyMimeType::Specific("image/webp".to_string()),
      _ => CopyMimeType::Autodetect,
    };
    sources.push(CopyMimeSource {
      source: CopySource::Bytes(image_data.data.to_vec().into_boxed_slice()),
      mime_type,
    });
  }

  if let Some(files) = contents.files {
    append_wayland_file_sources(&mut sources, &files);
  }

  if sources.is_empty() {
    clear()
  } else {
    wayland_copy_multi(sources)
  }
}

pub(crate) fn has_format(format: &str) -> WaylandResult<bool> {
  let available_formats = get_available_formats()?;
  Ok(has_wayland_format(&available_formats, format))
}

pub(crate) fn get_available_formats() -> WaylandResult<Vec<String>> {
  let offered_mimes = get_wayland_mime_types_ordered_or_empty()?;
  let mut formats = infer_wayland_available_formats(&offered_mimes);
  extend_wayland_formats_with_custom_mimes(&mut formats, &offered_mimes);

  Ok(formats)
}

pub(crate) fn clear() -> WaylandResult<()> {
  copy::clear(CopyClipboardType::Regular, CopySeat::All).map_err(|e| {
    format!(
      "Failed to clear clipboard: {}",
      wayland_copy_error_detail(&e)
    )
  })
}

pub(crate) fn get_full_clipboard_data() -> WaylandResult<ClipboardData> {
  let offered_mimes = get_wayland_mime_types_ordered_or_empty()?;
  Ok(read_wayland_complete_data_from_mimes(
    &offered_mimes,
    &mut |mime| {
      get_wayland_contents_bytes(PasteMimeType::Specific(mime)).map(|(payload, _)| payload)
    },
  ))
}

pub(crate) fn start_wayland_watch(
  tsfn: ThreadsafeFunction<ClipboardData, (), ClipboardData, napi::Status, false>,
  running: Arc<AtomicBool>,
) -> napi::Result<CancelHandle> {
  let (startup_tx, startup_rx) = mpsc::sync_channel(1);

  thread::Builder::new()
    .name("clipboard-wayland".to_string())
    .spawn(move || {
      let _state = ListenerState(Arc::clone(&running));
      let mut watcher = match Watcher::new(ClipboardType::Regular, PasteSeat::Unspecified) {
        Ok(watcher) => watcher,
        Err(e) => {
          let _ = startup_tx.send(Err(format!(
            "Failed to start Wayland clipboard listener: {e}"
          )));
          return;
        }
      };
      if startup_tx.send(Ok(watcher.cancel_handle())).is_err() {
        return;
      }

      while running.load(Ordering::Acquire) {
        let clipboard_data = match watcher.next_event() {
          Ok(Some(ClipboardEvent::Changed {
            mime_types,
            mut offer,
            ..
          })) => {
            // Read all formats from the event's offer to avoid mixing different selections.
            read_wayland_complete_data_from_mimes(&mime_types, &mut |mime| {
              let mut pipe = offer
                .receive(mime)
                .map_err(|e| format!("Failed to receive {mime}: {e}"))?;
              let mut payload = Vec::new();
              pipe
                .read_to_end(&mut payload)
                .map_err(|e| format!("Failed to read {mime}: {e}"))?;
              Ok(payload)
            })
          }
          Ok(Some(ClipboardEvent::Cleared { .. })) => {
            read_wayland_complete_data_from_mimes(&[], &mut |_| unreachable!())
          }
          Ok(None) => break,
          Err(e) => {
            wayland_log!("Wayland clipboard listener stopped: {e}");
            break;
          }
        };
        if !running.load(Ordering::Acquire) {
          break;
        }
        let status = tsfn.call(clipboard_data, ThreadsafeFunctionCallMode::NonBlocking);
        if status != napi::Status::Ok {
          wayland_log!("Wayland clipboard callback failed: {status:?}");
          break;
        }
      }
    })
    .map_err(|e| napi::Error::from_reason(format!("Failed to spawn Wayland listener: {e}")))?;

  startup_rx
    .recv()
    .map_err(|e| napi::Error::from_reason(format!("Wayland listener startup interrupted: {e}")))?
    .map_err(napi::Error::from_reason)
}
