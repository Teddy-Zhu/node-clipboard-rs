use crate::{ClipboardData, ImageData};
use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use std::sync::mpsc;
use std::thread;
use wayland_clipboard_listener::{
  ClipBoardListenMessage, WlClipboardListenerError, WlClipboardPasteStream, WlListenType,
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
    } else if let Ok(text_content) = String::from_utf8(payload) {
      wayland_log!(
        "wayland classified as text: selected_mime={}, chars={}",
        mime_type,
        text_content.chars().count()
      );
      text = Some(text_content);
    } else {
      wayland_log!("wayland text decode failed");
    }
  } else if is_wayland_html_mime(mime_type.as_str()) {
    push_wayland_format(&mut available_formats, "html");
    if let Ok(html_content) = String::from_utf8(payload) {
      wayland_log!(
        "wayland classified as html: selected_mime={}, chars={}",
        mime_type,
        html_content.chars().count()
      );
      html = Some(html_content);
    } else {
      wayland_log!("wayland html decode failed");
    }
  } else if is_wayland_rtf_mime(mime_type.as_str()) {
    push_wayland_format(&mut available_formats, "rtf");
    if let Ok(rtf_content) = String::from_utf8(payload) {
      wayland_log!(
        "wayland classified as rtf: selected_mime={}, chars={}",
        mime_type,
        rtf_content.chars().count()
      );
      rtf = Some(rtf_content);
    } else {
      wayland_log!("wayland rtf decode failed");
    }
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
  } else if let Ok(text_content) = String::from_utf8(payload) {
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
          wayland_log!(
            "watch_wayland event #{} raw message: selected_mime={}, offered_mimes={:?}, bytes={}",
            event_index,
            message.context.mime_type,
            message.mime_types,
            message.context.context.len()
          );

          let clipboard_data = wayland_context_to_clipboard_data(message);
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
