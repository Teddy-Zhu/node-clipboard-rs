use crate::{ClipboardData, ImageData};
use clipboard_rs::common::{RustImage, RustImageData};
use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use std::io::Read;
use std::sync::mpsc;
use std::thread;
use wayland_clipboard_listener::{
  ClipBoardListenMessage, WlClipboardListenerError, WlClipboardPasteStream, WlListenType,
};
use wl_clipboard_rs::copy::{
  self, ClipboardType as CopyClipboardType, Error as CopyError, MimeSource as CopyMimeSource,
  MimeType as CopyMimeType, Options as CopyOptions, Seat as CopySeat, Source as CopySource,
};
use wl_clipboard_rs::paste::{
  self, ClipboardType as PasteClipboardType, Error as PasteError, MimeType as PasteMimeType,
  Seat as PasteSeat,
};

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

fn wayland_error_detail(err: &WlClipboardListenerError) -> String {
  match err {
    WlClipboardListenerError::InitFailed(msg) => format!("InitFailed({msg})"),
    WlClipboardListenerError::QueueError(msg) => format!("QueueError({msg})"),
    WlClipboardListenerError::DispatchError(msg) => format!("DispatchError({msg})"),
    WlClipboardListenerError::PipeError => "PipeError".to_string(),
  }
}

pub(crate) fn is_wayland_environment() -> bool {
  if std::env::var("WAYLAND_DISPLAY").is_ok() {
    return true;
  }

  if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
    if session_type == "wayland" {
      return true;
    }
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

  match WlClipboardPasteStream::init(WlListenType::ListenOnCopy) {
    Ok(_) => {
      wayland_log!("is_wayland_clipboard_available=true");
      true
    }
    Err(e) => {
      wayland_log!(
        "is_wayland_clipboard_available=false: init failed: {}",
        wayland_error_detail(&e)
      );
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

fn decode_wayland_files(payload: &[u8]) -> Option<Vec<String>> {
  let file_list = String::from_utf8(payload.to_vec()).ok()?;
  let mut files = Vec::new();

  for line in file_list.lines() {
    let item = line.trim();
    if item.is_empty() || item.starts_with('#') {
      continue;
    }

    if item.eq_ignore_ascii_case("copy") || item.eq_ignore_ascii_case("cut") {
      continue;
    }

    files.push(item.to_string());
  }

  if files.is_empty() {
    None
  } else {
    Some(files)
  }
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

fn wayland_payload_head_hex(payload: &[u8], max_bytes: usize) -> String {
  payload
    .iter()
    .take(max_bytes)
    .map(|byte| format!("{byte:02x}"))
    .collect::<Vec<_>>()
    .join(" ")
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
) -> Option<String> {
  let selected_mime = find_wayland_mime(offered_mimes, preferred).or_else(|| {
    offered_mimes
      .iter()
      .find(|mime| fallback_predicate(mime.as_str()))
      .map(|mime| mime.as_str())
  })?;
  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Specific(selected_mime)).ok()?;
  Some(decode_utf8_payload_lossy(
    payload,
    format_name,
    selected_mime,
  ))
}

fn read_wayland_image_content(offered_mimes: &[String]) -> Option<ImageData> {
  let selected_mime =
    find_wayland_mime(offered_mimes, WAYLAND_IMAGE_MIME_PRIORITY).or_else(|| {
      offered_mimes
        .iter()
        .find(|mime| is_wayland_image_mime(mime))
        .map(|mime| mime.as_str())
    })?;
  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Specific(selected_mime)).ok()?;
  Some(to_wayland_image_data(payload))
}

fn read_wayland_files_content(offered_mimes: &[String]) -> Option<Vec<String>> {
  let selected_mime =
    find_wayland_mime(offered_mimes, WAYLAND_FILES_MIME_PRIORITY).or_else(|| {
      offered_mimes
        .iter()
        .find(|mime| is_wayland_files_mime(mime))
        .map(|mime| mime.as_str())
    })?;
  let (payload, _) = get_wayland_contents_bytes(PasteMimeType::Specific(selected_mime)).ok()?;
  decode_wayland_files(&payload)
}

fn read_wayland_complete_data_from_mimes(offered_mimes: &[String]) -> ClipboardData {
  let mut available_formats = infer_wayland_available_formats(offered_mimes);
  extend_wayland_formats_with_custom_mimes(&mut available_formats, offered_mimes);

  let text = if has_wayland_format(&available_formats, "text") {
    read_wayland_textual_content(
      offered_mimes,
      WAYLAND_TEXT_MIME_PRIORITY,
      is_wayland_text_mime,
      "text",
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
    )
  } else {
    None
  };

  let image = if has_wayland_format(&available_formats, "image") {
    read_wayland_image_content(offered_mimes)
  } else {
    None
  };

  let files = if has_wayland_format(&available_formats, "files") {
    read_wayland_files_content(offered_mimes)
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

fn to_wayland_image_data(payload: Vec<u8>) -> ImageData {
  match RustImageData::from_bytes(&payload) {
    Ok(image_data) => {
      let (width, height) = image_data.get_size();
      match image_data.to_png() {
        Ok(png_data) => {
          let bytes = png_data.get_bytes();
          ImageData {
            width,
            height,
            size: bytes.len() as u32,
            data: Buffer::from(bytes.to_vec()),
          }
        }
        Err(_) => ImageData {
          width,
          height,
          size: payload.len() as u32,
          data: Buffer::from(payload),
        },
      }
    }
    Err(_) => ImageData {
      width: 0,
      height: 0,
      size: payload.len() as u32,
      data: Buffer::from(payload),
    },
  }
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
    .ok_or_else(|| "Failed to decode clipboard files payload".to_string())
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

pub(crate) fn set_contents(contents: ClipboardData) -> WaylandResult<()> {
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
  Ok(read_wayland_complete_data_from_mimes(&offered_mimes))
}

fn wayland_context_to_clipboard_data(message: ClipBoardListenMessage) -> ClipboardData {
  let ClipBoardListenMessage {
    mime_types: offered_mime_types,
    context,
  } = message;
  let mime_type = context.mime_type;
  let payload = context.context;
  let payload_len = payload.len();
  let payload_magic = detect_wayland_image_magic(&payload);
  let payload_head = wayland_payload_head_hex(&payload, 16);
  let offered_image_mimes: Vec<&str> = offered_mime_types
    .iter()
    .filter(|mime| is_wayland_image_mime(mime.as_str()))
    .map(|mime| mime.as_str())
    .collect();

  wayland_log!(
    "wayland message received: selected_mime={}, offered_mimes={:?}, bytes={}, payload_head=[{}], payload_magic={:?}",
    mime_type,
    offered_mime_types,
    payload_len,
    payload_head,
    payload_magic
  );
  if !is_wayland_image_mime(mime_type.as_str()) && !offered_image_mimes.is_empty() {
    wayland_log!(
      "wayland possible mime mismatch: selected_mime={} but offered image mimes exist={:?}",
      mime_type,
      offered_image_mimes
    );
  }
  if payload_magic.is_some() && !is_wayland_image_mime(mime_type.as_str()) {
    wayland_log!(
      "wayland suspicious payload: selected_mime={} but payload looks like image({:?})",
      mime_type,
      payload_magic
    );
  }

  let mut available_formats = infer_wayland_available_formats(&offered_mime_types);
  extend_wayland_formats_with_custom_mimes(&mut available_formats, &offered_mime_types);
  let mut text = None;
  let mut rtf = None;
  let mut html = None;
  let mut image = None;
  let mut files = None;

  if is_wayland_text_mime(mime_type.as_str()) {
    push_wayland_format(&mut available_formats, "text");
    if payload_len == 0 && has_wayland_format(&available_formats, "image") {
      wayland_log!(
        "wayland text payload ignored: selected_mime={}, bytes=0, inferred_formats={:?}",
        mime_type,
        available_formats
      );
    } else {
      let text_content = decode_utf8_payload_lossy(payload, "text", &mime_type);
      wayland_log!(
        "wayland classified as text: selected_mime={}, chars={}",
        mime_type,
        text_content.chars().count()
      );
      text = Some(text_content);
    }
  } else if is_wayland_html_mime(mime_type.as_str()) {
    push_wayland_format(&mut available_formats, "html");
    let html_content = decode_utf8_payload_lossy(payload, "html", &mime_type);
    wayland_log!(
      "wayland classified as html: selected_mime={}, chars={}",
      mime_type,
      html_content.chars().count()
    );
    html = Some(html_content);
  } else if is_wayland_rtf_mime(mime_type.as_str()) {
    push_wayland_format(&mut available_formats, "rtf");
    let rtf_content = decode_utf8_payload_lossy(payload, "rich text", &mime_type);
    wayland_log!(
      "wayland classified as rtf: selected_mime={}, chars={}",
      mime_type,
      rtf_content.chars().count()
    );
    rtf = Some(rtf_content);
  } else if is_wayland_image_mime(mime_type.as_str()) {
    push_wayland_format(&mut available_formats, "image");
    wayland_log!(
      "wayland classified as image: selected_mime={}, bytes={}, payload_magic={:?}",
      mime_type,
      payload_len,
      payload_magic
    );
    image = Some(ImageData {
      width: 0,
      height: 0,
      size: payload_len as u32,
      data: Buffer::from(payload),
    });
  } else if is_wayland_files_mime(mime_type.as_str()) {
    push_wayland_format(&mut available_formats, "files");
    files = decode_wayland_files(&payload);
    wayland_log!(
      "wayland classified as files: selected_mime={}, count={}",
      mime_type,
      files.as_ref().map(|list| list.len()).unwrap_or(0)
    );
  } else if std::str::from_utf8(&payload).is_ok() {
    let text_content = decode_utf8_payload_lossy(payload, "text", &mime_type);
    push_wayland_format(&mut available_formats, "text");
    wayland_log!(
      "wayland fallback to text: unsupported selected_mime={}, chars={}, offered_mimes={:?}",
      mime_type,
      text_content.chars().count(),
      offered_mime_types
    );
    text = Some(text_content);
  } else {
    wayland_log!(
      "wayland unsupported mime_type and utf8 decode failed: mime_type={}",
      mime_type
    );
  }

  wayland_log!(
    "wayland conversion result: selected_mime={}, available_formats={:?}, has_text={}, has_rtf={}, has_html={}, has_image={}, has_files={}",
    mime_type,
    available_formats,
    text.is_some(),
    rtf.is_some(),
    html.is_some(),
    image.is_some(),
    files.is_some()
  );

  ClipboardData {
    available_formats,
    text,
    rtf,
    html,
    image,
    files,
  }
}

fn merge_wayland_clipboard_data(
  mut primary: ClipboardData,
  fallback: ClipboardData,
) -> ClipboardData {
  if primary.available_formats.is_empty() {
    primary.available_formats = fallback.available_formats;
  }
  if primary.text.is_none() {
    primary.text = fallback.text;
  }
  if primary.rtf.is_none() {
    primary.rtf = fallback.rtf;
  }
  if primary.html.is_none() {
    primary.html = fallback.html;
  }
  if primary.image.is_none() {
    primary.image = fallback.image;
  }
  if primary.files.is_none() {
    primary.files = fallback.files;
  }
  primary
}

pub(crate) fn start_wayland_watch(
  tsfn: ThreadsafeFunction<ClipboardData, (), ClipboardData, napi::Status, false>,
) -> mpsc::Sender<()> {
  let (stop_tx, stop_rx) = mpsc::channel::<()>();

  thread::spawn(move || {
    wayland_log!("watch_wayland thread started");

    let mut stream = match WlClipboardPasteStream::init(WlListenType::ListenOnCopy) {
      Ok(stream) => {
        wayland_log!("watch_wayland stream initialized");
        stream
      }
      Err(e) => {
        wayland_log!(
          "watch_wayland stream init failed: {}",
          wayland_error_detail(&e)
        );
        return;
      }
    };

    let priority = vec![
      "image/png".into(),
      "image/jpeg".into(),
      "image/webp".into(),
      "image/bmp".into(),
      "image/gif".into(),
      "application/x-qt-image".into(),
      "text/rtf".into(),
      "application/rtf".into(),
      "application/x-rtf".into(),
      "text/richtext".into(),
      "text/uri-list".into(),
      "x-special/gnome-copied-files".into(),
      "x-special/nautilus-clipboard".into(),
      "text/html".into(),
      "text/plain;charset=utf-8".into(),
      "text/plain".into(),
    ];
    stream.set_priority(priority.clone());
    wayland_log!("watch_wayland stream priority configured: {:?}", priority);

    let mut event_index: u64 = 0;
    for context_result in stream.paste_stream() {
      if stop_rx.try_recv().is_ok() {
        wayland_log!("watch_wayland received stop signal");
        break;
      }

      match context_result {
        Ok(message) => {
          event_index += 1;
          let selected_mime = message.context.mime_type.clone();
          let offered_mimes = message.mime_types.clone();
          let payload_len = message.context.context.len();
          wayland_log!(
            "watch_wayland event #{} raw message: selected_mime={}, offered_mimes={:?}, bytes={}",
            event_index,
            selected_mime,
            offered_mimes,
            payload_len
          );

          let complete_data = read_wayland_complete_data_from_mimes(&offered_mimes);
          let fallback_data = wayland_context_to_clipboard_data(message);
          let clipboard_data = merge_wayland_clipboard_data(complete_data, fallback_data);
          wayland_log!(
            "watch_wayland event #{} normalized result: available_formats={:?}, has_text={}, has_rtf={}, has_html={}, has_image={}, has_files={}",
            event_index,
            clipboard_data.available_formats,
            clipboard_data.text.is_some(),
            clipboard_data.rtf.is_some(),
            clipboard_data.html.is_some(),
            clipboard_data.image.is_some(),
            clipboard_data.files.is_some()
          );

          let status = tsfn.call(clipboard_data, ThreadsafeFunctionCallMode::NonBlocking);
          if status == napi::Status::Ok {
            wayland_log!(
              "watch_wayland callback dispatched for event #{}",
              event_index
            );
          } else {
            wayland_log!(
              "watch_wayland callback dispatch failed: event=#{}, status={status:?}",
              event_index
            );
          }
        }
        Err(e) => {
          wayland_log!(
            "watch_wayland stream yielded error: {}",
            wayland_error_detail(&e)
          );
        }
      }
    }

    wayland_log!("watch_wayland loop exited");
  });

  stop_tx
}
