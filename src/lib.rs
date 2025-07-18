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
  /// 图片数据（base64 编码）
  pub image: Option<String>,
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
          image = context
            .get_image()
            .ok()
            .and_then(|img_data| img_data.to_png().ok())
            .map(|png_data| BASE64_STANDARD.encode(png_data.get_bytes()));
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

/// 剪贴板监听器实例，用于监听剪贴板变化并支持停止
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
  shutdown: Option<clipboard_rs::WatcherShutdown>,
}

#[napi]
impl ClipboardListener {
  /// 创建新的剪贴板监听器实例
  #[napi(constructor)]
  pub fn new() -> Result<Self> {
    Ok(ClipboardListener { shutdown: None })
  }

  /// 开始监听剪贴板变化
  /// callback: 当剪贴板变化时调用的回调函数，参数为包含所有格式数据的复杂对象
  #[napi]
  pub fn watch(&mut self, callback: Function<ClipboardData, ()>) -> Result<()> {
    // 如果已经在监听，先停止
    if self.shutdown.is_some() {
      self.stop()?;
    }

    // 创建线程安全的函数
    let tsfn = callback
      .build_threadsafe_function()
      .build_callback(|ctx| Ok(ctx.value))?;

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
      self.shutdown = Some(shutdown);
    }

    Ok(())
  }

  /// 停止监听剪贴板变化
  #[napi]
  pub fn stop(&mut self) -> Result<()> {
    if let Some(shutdown) = self.shutdown.take() {
      shutdown.stop();
    }
    Ok(())
  }

  /// 检查是否正在监听
  #[napi]
  pub fn is_watching(&self) -> bool {
    self.shutdown.is_some()
  }
}
