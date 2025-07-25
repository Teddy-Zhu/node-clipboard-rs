#![deny(clippy::all)]

use base64::prelude::*;
use clipboard_rs::common::{RustImage, RustImageData};
use clipboard_rs::{
  Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext,
  ContentFormat,
};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::thread;

// 仅在 Linux 下导入 Wayland 相关依赖
#[cfg(target_os = "linux")]
use wayland_clipboard_listener::{ClipBoardListenMessage, WlClipboardPasteStream, WlListenType};

/// 检测当前环境是否为 Wayland
#[cfg(target_os = "linux")]
fn is_wayland_environment() -> bool {
  // 检查 WAYLAND_DISPLAY 环境变量
  if std::env::var("WAYLAND_DISPLAY").is_ok() {
    return true;
  }

  // 检查 XDG_SESSION_TYPE 环境变量
  if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
    if session_type == "wayland" {
      return true;
    }
  }

  false
}

/// 非 Linux 平台的 Wayland 环境检测（总是返回 false）
#[cfg(not(target_os = "linux"))]
fn is_wayland_environment() -> bool {
  false
}

/// 检测 Wayland 剪贴板监听是否可用
///
/// 返回 true 表示当前环境支持 Wayland 剪贴板监听
#[napi]
pub fn is_wayland_clipboard_available() -> bool {
  #[cfg(target_os = "linux")]
  {
    if !is_wayland_environment() {
      return false;
    }

    // 尝试初始化 Wayland 剪贴板流来测试是否可用
    WlClipboardPasteStream::init(WlListenType::ListenOnCopy).is_ok()
  }

  #[cfg(not(target_os = "linux"))]
  {
    false
  }
}

/// 图片数据结构，包含图片的详细信息
#[napi(object)]
pub struct ImageData {
  /// 图片宽度（像素）
  pub width: u32,
  /// 图片高度（像素）
  pub height: u32,
  /// 图片数据大小（字节）
  pub size: u32,
  /// 图片原始数据（Buffer）
  pub data: Buffer,
}

/// 剪贴板数据结构，包含所有可用格式的数据
#[napi(object)]
pub struct ClipboardData {
  /// 可用的格式列表
  pub available_formats: Vec<String>,
  /// 纯文本内容
  pub text: Option<String>,
  /// RTF 富文本内容
  pub rtf: Option<String>,
  /// HTML 内容
  pub html: Option<String>,
  /// 图片数据
  pub image: Option<ImageData>,
  /// 文件列表
  pub files: Option<Vec<String>>,
}

/// 剪贴板管理器，提供跨平台的剪贴板操作功能
#[napi]
pub struct ClipboardManager {
  context: ClipboardContext,
}

#[napi]
impl ClipboardManager {
  /// 创建新的剪贴板管理器实例
  #[napi(constructor)]
  pub fn new() -> Result<Self> {
    let context = ClipboardContext::new().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create clipboard context: {e}"),
      )
    })?;

    Ok(ClipboardManager { context })
  }

  /// 获取剪贴板中的纯文本内容
  #[napi]
  pub fn get_text(&self) -> Result<String> {
    self
      .context
      .get_text()
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get text: {e}")))
  }

  /// 设置剪贴板中的纯文本内容
  #[napi]
  pub fn set_text(&self, text: String) -> Result<()> {
    self
      .context
      .set_text(text)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set text: {e}")))
  }

  /// 获取剪贴板中的 HTML 内容
  #[napi]
  pub fn get_html(&self) -> Result<String> {
    self
      .context
      .get_html()
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get HTML: {e}")))
  }

  /// 设置剪贴板中的 HTML 内容
  #[napi]
  pub fn set_html(&self, html: String) -> Result<()> {
    self
      .context
      .set_html(html)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set HTML: {e}")))
  }

  /// 获取剪贴板中的富文本内容
  #[napi]
  pub fn get_rich_text(&self) -> Result<String> {
    self.context.get_rich_text().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to get rich text: {e}"),
      )
    })
  }

  /// 设置剪贴板中的富文本内容
  #[napi]
  pub fn set_rich_text(&self, text: String) -> Result<()> {
    self.context.set_rich_text(text).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to set rich text: {e}"),
      )
    })
  }

  /// 获取剪贴板中的图片数据（以 base64 编码返回）
  #[napi]
  pub fn get_image_base64(&self) -> Result<String> {
    let image_data = self
      .context
      .get_image()
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

    let png_data = image_data.to_png().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to convert image to PNG: {e}"),
      )
    })?;

    Ok(BASE64_STANDARD.encode(png_data.get_bytes()))
  }

  /// 获取剪贴板中的图片详细信息（包含宽度、高度、大小和原始数据）
  #[napi]
  pub fn get_image_data(&self) -> Result<ImageData> {
    let image_data = self
      .context
      .get_image()
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

    let (width, height) = image_data.get_size();
    let png_data = image_data.to_png().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to convert image to PNG: {e}"),
      )
    })?;

    let bytes = png_data.get_bytes();
    Ok(ImageData {
      width,
      height,
      size: bytes.len() as u32,
      data: Buffer::from(bytes.to_vec()),
    })
  }

  /// 从 base64 编码的图片数据设置剪贴板图片
  #[napi]
  pub fn set_image_base64(&self, base64_data: String) -> Result<()> {
    let image_data = BASE64_STANDARD
      .decode(base64_data)
      .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid base64 data: {e}")))?;

    let rust_image = RustImageData::from_bytes(&image_data).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create image from bytes: {e}"),
      )
    })?;

    self
      .context
      .set_image(rust_image)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set image: {e}")))
  }

  /// 从原始字节数据设置剪贴板图片
  #[napi]
  pub fn set_image_raw(&self, image_data: Buffer) -> Result<()> {
    let rust_image = RustImageData::from_bytes(&image_data).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create image from bytes: {e}"),
      )
    })?;

    self
      .context
      .set_image(rust_image)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set image: {e}")))
  }

  /// 获取剪贴板中的图片原始数据（Buffer）
  #[napi]
  pub fn get_image_raw(&self) -> Result<Buffer> {
    let image_data = self
      .context
      .get_image()
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

    let png_data = image_data.to_png().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to convert image to PNG: {e}"),
      )
    })?;

    Ok(Buffer::from(png_data.get_bytes().to_vec()))
  }

  /// 获取剪贴板中的文件列表
  #[napi]
  pub fn get_files(&self) -> Result<Vec<String>> {
    self
      .context
      .get_files()
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get files: {e}")))
  }

  /// 设置剪贴板中的文件列表
  #[napi]
  pub fn set_files(&self, files: Vec<String>) -> Result<()> {
    self
      .context
      .set_files(files)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set files: {e}")))
  }

  /// 设置剪贴板中的自定义格式数据
  #[napi]
  pub fn set_buffer(&self, format: String, buffer: Buffer) -> Result<()> {
    self
      .context
      .set_buffer(&format, buffer.to_vec())
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set buffer: {e}")))
  }

  /// 获取剪贴板中的自定义格式数据
  #[napi]
  pub fn get_buffer(&self, format: String) -> Result<Buffer> {
    let data = self
      .context
      .get_buffer(&format)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get buffer: {e}")))?;
    Ok(Buffer::from(data))
  }

  /// 设置剪贴板中的复合内容（可同时设置多种格式）
  #[napi]
  pub fn set_contents(&self, contents: ClipboardData) -> Result<()> {
    use clipboard_rs::ClipboardContent;

    let mut clipboard_contents = Vec::new();

    if let Some(text) = contents.text {
      clipboard_contents.push(ClipboardContent::Text(text));
    }

    if let Some(html) = contents.html {
      clipboard_contents.push(ClipboardContent::Html(html));
    }

    if let Some(rtf) = contents.rtf {
      clipboard_contents.push(ClipboardContent::Rtf(rtf));
    }

    if let Some(image_data) = contents.image {
      let rust_image = RustImageData::from_bytes(image_data.data.as_ref()).map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to create image from bytes: {e}"),
        )
      })?;

      clipboard_contents.push(ClipboardContent::Image(rust_image));
    }

    if let Some(files) = contents.files {
      clipboard_contents.push(ClipboardContent::Files(files));
    }

    self.context.set(clipboard_contents).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to set contents: {e}"),
      )
    })
  }

  /// 检查剪贴板是否包含指定格式的内容
  #[napi]
  pub fn has_format(&self, format: String) -> Result<bool> {
    let content_format = match format.as_str() {
      "text" => ContentFormat::Text,
      "html" => ContentFormat::Html,
      "rtf" | "rich_text" => ContentFormat::Rtf,
      "image" => ContentFormat::Image,
      "files" => ContentFormat::Files,
      _ => {
        return Err(Error::new(
          Status::InvalidArg,
          format!("Unsupported format: {format}"),
        ))
      }
    };

    Ok(self.context.has(content_format))
  }

  /// 获取剪贴板中所有可用的格式
  #[napi]
  pub fn get_available_formats(&self) -> Result<Vec<String>> {
    self.context.available_formats().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to get available formats: {e}"),
      )
    })
  }

  /// 清空剪贴板
  #[napi]
  pub fn clear(&self) -> Result<()> {
    self.context.clear().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to clear clipboard: {e}"),
      )
    })
  }

  /// 异步获取剪贴板文本内容
  #[napi]
  pub async fn get_text_async(&self) -> Result<String> {
    let context = ClipboardContext::new().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create clipboard context: {e}"),
      )
    })?;

    tokio::task::spawn_blocking(move || {
      context
        .get_text()
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get text: {e}")))
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {e}")))?
  }

  /// 异步设置剪贴板文本内容
  #[napi]
  pub async fn set_text_async(&self, text: String) -> Result<()> {
    let context = ClipboardContext::new().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create clipboard context: {e}"),
      )
    })?;

    tokio::task::spawn_blocking(move || {
      context
        .set_text(text)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set text: {e}")))
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {e}")))?
  }

  /// 异步获取剪贴板图片数据（以 base64 编码返回）
  #[napi]
  pub async fn get_image_base64_async(&self) -> Result<String> {
    let context = ClipboardContext::new().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create clipboard context: {e}"),
      )
    })?;

    tokio::task::spawn_blocking(move || {
      let image_data = context
        .get_image()
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

      let png_data = image_data.to_png().map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to convert image to PNG: {e}"),
        )
      })?;

      Ok(BASE64_STANDARD.encode(png_data.get_bytes()))
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {e}")))?
  }

  /// 异步获取剪贴板图片详细信息（包含宽度、高度、大小和原始数据）
  #[napi]
  pub async fn get_image_data_async(&self) -> Result<ImageData> {
    let context = ClipboardContext::new().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create clipboard context: {e}"),
      )
    })?;

    tokio::task::spawn_blocking(move || {
      let image_data = context
        .get_image()
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

      let (width, height) = image_data.get_size();
      let png_data = image_data.to_png().map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to convert image to PNG: {e}"),
        )
      })?;

      let bytes = png_data.get_bytes();
      Ok(ImageData {
        width,
        height,
        size: bytes.len() as u32,
        data: Buffer::from(bytes.to_vec()),
      })
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {e}")))?
  }
}

// 便利的静态函数，用于快速操作剪贴板

/// 快速获取剪贴板文本内容
#[napi]
pub fn get_clipboard_text() -> Result<String> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context
    .get_text()
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get text: {e}")))
}

/// 快速设置剪贴板文本内容
#[napi]
pub fn set_clipboard_text(text: String) -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context
    .set_text(text)
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set text: {e}")))
}

/// 快速获取剪贴板 HTML 内容
#[napi]
pub fn get_clipboard_html() -> Result<String> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context
    .get_html()
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get HTML: {e}")))
}

/// 快速设置剪贴板 HTML 内容
#[napi]
pub fn set_clipboard_html(html: String) -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context
    .set_html(html)
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set HTML: {e}")))
}

/// 快速获取剪贴板图片（base64 编码）
#[napi]
pub fn get_clipboard_image() -> Result<String> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  let image_data = context
    .get_image()
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

  let png_data = image_data.to_png().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to convert image to PNG: {e}"),
    )
  })?;

  Ok(BASE64_STANDARD.encode(png_data.get_bytes()))
}

/// 快速获取剪贴板图片详细信息（包含宽度、高度、大小和原始数据）
#[napi]
pub fn get_clipboard_image_data() -> Result<ImageData> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  let image_data = context
    .get_image()
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

  let (width, height) = image_data.get_size();
  let png_data = image_data.to_png().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to convert image to PNG: {e}"),
    )
  })?;

  let bytes = png_data.get_bytes();
  Ok(ImageData {
    width,
    height,
    size: bytes.len() as u32,
    data: Buffer::from(bytes.to_vec()),
  })
}

/// 快速设置剪贴板图片（从 base64 编码）
#[napi]
pub fn set_clipboard_image(base64_data: String) -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  let image_data = BASE64_STANDARD
    .decode(base64_data)
    .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid base64 data: {e}")))?;

  let rust_image = RustImageData::from_bytes(&image_data).map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create image from bytes: {e}"),
    )
  })?;

  context
    .set_image(rust_image)
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set image: {e}")))
}

/// 快速设置剪贴板图片（从原始字节数据）
#[napi]
pub fn set_clipboard_image_raw(image_data: Buffer) -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  let rust_image = RustImageData::from_bytes(&image_data).map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create image from bytes: {e}"),
    )
  })?;

  context
    .set_image(rust_image)
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set image: {e}")))
}

/// 快速获取剪贴板图片原始数据（Buffer）
#[napi]
pub fn get_clipboard_image_raw() -> Result<Buffer> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  let image_data = context
    .get_image()
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get image: {e}")))?;

  let png_data = image_data.to_png().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to convert image to PNG: {e}"),
    )
  })?;

  Ok(Buffer::from(png_data.get_bytes().to_vec()))
}

/// 快速设置剪贴板自定义格式数据
#[napi]
pub fn set_clipboard_buffer(format: String, buffer: Buffer) -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context
    .set_buffer(&format, buffer.to_vec())
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set buffer: {e}")))
}

/// 快速获取剪贴板自定义格式数据
#[napi]
pub fn get_clipboard_buffer(format: String) -> Result<Buffer> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  let data = context
    .get_buffer(&format)
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get buffer: {e}")))?;
  Ok(Buffer::from(data))
}

/// 快速设置剪贴板文件列表
#[napi]
pub fn set_clipboard_files(files: Vec<String>) -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context
    .set_files(files)
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to set files: {e}")))
}

/// 快速获取剪贴板文件列表
#[napi]
pub fn get_clipboard_files() -> Result<Vec<String>> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context
    .get_files()
    .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get files: {e}")))
}

/// 快速设置剪贴板复合内容（可同时设置多种格式）
#[napi]
pub fn set_clipboard_contents(contents: ClipboardData) -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  use clipboard_rs::ClipboardContent;

  let mut clipboard_contents = Vec::new();

  if let Some(text) = contents.text {
    clipboard_contents.push(ClipboardContent::Text(text));
  }

  if let Some(html) = contents.html {
    clipboard_contents.push(ClipboardContent::Html(html));
  }

  if let Some(rtf) = contents.rtf {
    clipboard_contents.push(ClipboardContent::Rtf(rtf));
  }

  if let Some(image_data) = contents.image {
    let rust_image = RustImageData::from_bytes(image_data.data.as_ref()).map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to create image from bytes: {e}"),
      )
    })?;

    clipboard_contents.push(ClipboardContent::Image(rust_image));
  }

  if let Some(files) = contents.files {
    clipboard_contents.push(ClipboardContent::Files(files));
  }

  context.set(clipboard_contents).map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to set contents: {e}"),
    )
  })
}

/// 快速获取完整的剪贴板数据
#[napi]
pub fn get_full_clipboard_data() -> Result<ClipboardData> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  Ok(get_clipboard_data(&context))
}

/// 快速清空剪贴板
#[napi]
pub fn clear_clipboard() -> Result<()> {
  let context = ClipboardContext::new().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to create clipboard context: {e}"),
    )
  })?;

  context.clear().map_err(|e| {
    Error::new(
      Status::GenericFailure,
      format!("Failed to clear clipboard: {e}"),
    )
  })
}

/// 将 Wayland 剪贴板消息转换为我们的 ClipboardData 格式
#[cfg(target_os = "linux")]
fn wayland_context_to_clipboard_data(message: ClipBoardListenMessage) -> ClipboardData {
  let mut available_formats = Vec::new();
  let mut text = None;
  let mut html = None;
  let mut image = None;

  // 根据 MIME 类型处理数据
  match message.context.mime_type.as_str() {
    "text/plain" | "text/plain;charset=utf-8" | "UTF8_STRING" | "STRING" => {
      available_formats.push("text".to_string());
      if let Ok(text_content) = String::from_utf8(message.context.context) {
        text = Some(text_content);
      }
    }
    "text/html" => {
      available_formats.push("html".to_string());
      if let Ok(html_content) = String::from_utf8(message.context.context) {
        html = Some(html_content);
      }
    }
    "image/png" | "image/jpeg" | "image/gif" | "image/bmp" => {
      available_formats.push("image".to_string());
      // 由于 Wayland 监听器不能直接获取图片尺寸，我们设置为 0
      image = Some(ImageData {
        width: 0,
        height: 0,
        size: message.context.context.len() as u32,
        data: Buffer::from(message.context.context),
      });
    }
    _ => {
      // 对于其他类型，尝试作为文本处理
      if let Ok(text_content) = String::from_utf8(message.context.context) {
        available_formats.push("text".to_string());
        text = Some(text_content);
      }
    }
  }

  ClipboardData {
    available_formats,
    text,
    rtf: None, // Wayland 监听器暂不支持 RTF
    html,
    image,
    files: None, // Wayland 监听器暂不支持文件列表
  }
}

/// 获取完整的剪贴板数据
fn get_clipboard_data(context: &ClipboardContext) -> ClipboardData {
  // 定义要检查的格式类型，对应 ClipboardContent 枚举
  // Text, Rtf, Html, Image, Files
  let format_checks = [
    ("text", ContentFormat::Text),
    ("rtf", ContentFormat::Rtf),
    ("html", ContentFormat::Html),
    ("image", ContentFormat::Image),
    ("files", ContentFormat::Files),
  ];

  // 初始化数据变量
  let mut available_formats = Vec::new();
  let mut text = None;
  let mut rtf = None;
  let mut html = None;
  let mut image = None;
  let mut files = None;

  // 使用 has 接口检查每种标准格式的可用性并使用对应的 get 接口获取数据
  for (format_name, content_format) in format_checks.iter() {
    if context.has(content_format.clone()) {
      available_formats.push(format_name.to_string());

      match *format_name {
        "text" => {
          text = context.get_text().ok();
        }
        "rtf" => {
          rtf = context.get_rich_text().ok();
        }
        "html" => {
          html = context.get_html().ok();
        }
        "image" => {
          image = context.get_image().ok().and_then(|img_data| {
            let (width, height) = img_data.get_size();
            img_data.to_png().ok().map(|png_data| {
              let bytes = png_data.get_bytes();
              ImageData {
                width,
                height,
                size: bytes.len() as u32,
                data: Buffer::from(bytes.to_vec()),
              }
            })
          });
        }
        "files" => {
          files = context.get_files().ok();
        }
        _ => {}
      }
    }
  }

  ClipboardData {
    available_formats,
    text,
    rtf,
    html,
    image,
    files,
  }
}

/// 监听器类型枚举
enum ListenerType {
  /// 使用 clipboard_rs 监听器（X11/通用）
  ClipboardRs(clipboard_rs::WatcherShutdown),
  /// 使用 Wayland 专用监听器（仅 Linux）
  #[cfg(target_os = "linux")]
  Wayland(std::sync::mpsc::Sender<()>),
}

/// 剪贴板监听器实例，用于监听剪贴板变化并支持停止
/// 支持自动检测环境：在 Wayland 环境下使用 Wayland 专用监听器，否则使用通用监听器
/// 使用方法：
/// ```javascript
/// const { ClipboardListener } = require('./index.node');
/// const listener = new ClipboardListener();
/// listener.watch((data) => {
///   console.log('剪贴板数据变化:', data);
///   console.log('可用格式:', data.available_formats);
///   if (data.text) console.log('文本:', data.text);
///   if (data.html) console.log('HTML:', data.html);
///   if (data.rtf) console.log('RTF:', data.rtf);
///   if (data.image) console.log('图片 (base64):', data.image.substring(0, 50) + '...');
///   if (data.files) console.log('文件:', data.files);
///   if (data.other) console.log('其他格式:', data.other);
/// });
/// // 停止监听
/// listener.stop();
/// ```
#[napi]
pub struct ClipboardListener {
  listener_type: Option<ListenerType>,
  is_wayland: bool,
}

#[napi]
impl ClipboardListener {
  /// 创建新的剪贴板监听器实例
  /// 自动检测当前环境类型（Wayland 或其他）
  #[napi(constructor)]
  pub fn new() -> Result<Self> {
    let is_wayland = is_wayland_environment();
    Ok(ClipboardListener {
      listener_type: None,
      is_wayland,
    })
  }

  /// 开始监听剪贴板变化
  /// callback: 当剪贴板变化时调用的回调函数，参数为包含所有格式数据的复杂对象
  /// 自动根据当前环境选择合适的监听方式（Wayland 或通用）
  #[napi]
  pub fn watch(&mut self, callback: Function<ClipboardData, ()>) -> Result<()> {
    // 如果已经在监听，先停止
    if self.listener_type.is_some() {
      self.stop()?;
    }

    // 创建线程安全的函数
    let tsfn = callback
      .build_threadsafe_function()
      .build_callback(|ctx| Ok(ctx.value))?;

    if self.is_wayland {
      self.watch_wayland(tsfn)
    } else {
      self.watch_generic(tsfn)
    }
  }

  /// 使用 Wayland 专用监听器监听剪贴板变化
  #[cfg(target_os = "linux")]
  fn watch_wayland(
    &mut self,
    tsfn: ThreadsafeFunction<ClipboardData, (), ClipboardData, napi::Status, false>,
  ) -> Result<()> {
    // 创建停止信号通道
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();

    // 在新线程中启动 Wayland 剪贴板监听
    thread::spawn(move || {
      // 创建 Wayland 剪贴板流
      let mut stream = match WlClipboardPasteStream::init(WlListenType::ListenOnCopy) {
        Ok(stream) => stream,
        Err(_) => return,
      };

      // 设置 MIME 类型优先级
      stream.set_priority(vec![
        "text/plain;charset=utf-8".into(),
        "text/plain".into(),
        "text/html".into(),
        "image/png".into(),
        "image/jpeg".into(),
      ]);

      // 监听剪贴板变化
      for context_result in stream.paste_stream() {
        // 检查停止信号
        if stop_rx.try_recv().is_ok() {
          break;
        }

        if let Ok(Some(message)) = context_result {
          // 将 Wayland 剪贴板数据转换为我们的格式
          let clipboard_data = wayland_context_to_clipboard_data(message);
          let _ = tsfn.call(clipboard_data, ThreadsafeFunctionCallMode::NonBlocking);
        }
      }
    });

    // 保存停止通道
    self.listener_type = Some(ListenerType::Wayland(stop_tx));
    Ok(())
  }

  /// 非 Linux 平台的 Wayland 监听器（空实现）
  #[cfg(not(target_os = "linux"))]
  fn watch_wayland(
    &mut self,
    _tsfn: ThreadsafeFunction<ClipboardData, (), ClipboardData, napi::Status, false>,
  ) -> Result<()> {
    Err(Error::new(
      Status::GenericFailure,
      "Wayland clipboard listener is not supported on this platform".to_string(),
    ))
  }

  /// 使用通用监听器监听剪贴板变化
  fn watch_generic(
    &mut self,
    tsfn: ThreadsafeFunction<ClipboardData, (), ClipboardData, napi::Status, false>,
  ) -> Result<()> {
    // 创建通道用于传递 shutdown
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<clipboard_rs::WatcherShutdown>();

    // 在新线程中启动剪贴板监听
    thread::spawn(move || {
      // 创建剪贴板上下文
      let ctx = match ClipboardContext::new() {
        Ok(ctx) => ctx,
        Err(_) => return,
      };

      // 创建处理器
      struct Handler {
        ctx: ClipboardContext,
        callback: ThreadsafeFunction<ClipboardData, (), ClipboardData, napi::Status, false>,
      }

      impl ClipboardHandler for Handler {
        fn on_clipboard_change(&mut self) {
          let clipboard_data = get_clipboard_data(&self.ctx);
          let _ = self
            .callback
            .call(clipboard_data, ThreadsafeFunctionCallMode::NonBlocking);
        }
      }

      let handler = Handler {
        ctx,
        callback: tsfn,
      };

      // 创建监听器上下文
      let mut watcher = match ClipboardWatcherContext::new() {
        Ok(watcher) => watcher,
        Err(_) => return,
      };

      // 添加处理器并获取关闭通道
      let shutdown = watcher.add_handler(handler).get_shutdown_channel();

      // 将 shutdown 发送给主线程
      let _ = shutdown_tx.send(shutdown);

      // 启动监听
      watcher.start_watch();
    });

    // 接收 shutdown 并保存
    if let Ok(shutdown) = shutdown_rx.recv() {
      self.listener_type = Some(ListenerType::ClipboardRs(shutdown));
    }

    Ok(())
  }

  /// 停止监听剪贴板变化
  #[napi]
  pub fn stop(&mut self) -> Result<()> {
    if let Some(listener_type) = self.listener_type.take() {
      match listener_type {
        ListenerType::ClipboardRs(shutdown) => {
          shutdown.stop();
        }
        #[cfg(target_os = "linux")]
        ListenerType::Wayland(stop_tx) => {
          let _ = stop_tx.send(());
        }
      }
    }
    Ok(())
  }

  /// 检查是否正在监听
  #[napi]
  pub fn is_watching(&self) -> bool {
    self.listener_type.is_some()
  }

  /// 获取当前使用的监听器类型
  #[napi]
  pub fn get_listener_type(&self) -> String {
    if self.is_wayland {
      "wayland".to_string()
    } else {
      "generic".to_string()
    }
  }
}
