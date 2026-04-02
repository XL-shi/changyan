# SenseVoice Small 本地推理集成计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 集成 SenseVoice Small 本地推理，实现零配置、完全免费、离线可用的语音识别。

**Architecture:** 使用 `sherpa-onnx` Rust crate 调用预构建的 sherpa-onnx C++ 共享库，实现 `SttProvider` trait 进行本地推理。模型文件（~360MB ONNX）在首次使用时通过 Tauri IPC 命令下载到应用数据目录，前端提供下载进度 UI。Pipeline 对本地 provider 跳过 API key 校验。

**Tech Stack:**
- Rust: `sherpa-onnx` crate（链接预构建共享库）
- 模型源: GitHub releases / HuggingFace（SenseVoice Small ONNX）
- Frontend: React + Zustand，新增模型管理 UI in `SttPane.tsx`

---

## 背景与关键约束

### sherpa-onnx Rust 集成方式

`sherpa-onnx` crate 有两种链接方式：
- **动态链接预构建库**（推荐）：下载平台对应的 `.dylib`/`.dll`/`.so`，通过 `SHERPA_ONNX_LIB_DIR` 环境变量指定，构建快、CI 友好
- **从源码编译**：需要 cmake + C++ 编译器，构建时间长，不推荐

本计划采用**动态链接**方式。

### 模型文件清单（SenseVoice Small via sherpa-onnx）

| 文件 | 大小 | 用途 |
|---|---|---|
| `model.int8.onnx` | ~234MB | 量化推理模型（推荐，内存友好） |
| `tokens.txt` | ~100KB | 词表 |
| `README.md` | 极小 | 可选 |

下载源：`https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models`

### 模型存储路径

使用 Tauri 的 `app_data_dir()`：
- macOS: `~/Library/Application Support/changyan/models/sensevoice-small/`
- Windows: `%APPDATA%\changyan\models\sensevoice-small\`

### Pipeline 改动点

`pipeline.rs` 第 312-332 行有 API key 为空时的 bail 逻辑：
```rust
if config_data.stt_api_key.is_empty() && config_data.stt_provider != "cloud" {
```
需要改为：
```rust
if config_data.stt_api_key.is_empty()
    && config_data.stt_provider != "cloud"
    && config_data.stt_provider != "sensevoice-local"
{
```

---

## Task 1: 调研并确认 sherpa-onnx crate API

**Files:**
- Read: `src-tauri/Cargo.toml`
- 参考: https://crates.io/crates/sherpa-onnx

**Step 1: 查看 sherpa-onnx 最新版本和 API**

```bash
cargo search sherpa-onnx
```

**Step 2: 验证 SenseVoice API 签名**

sherpa-onnx 的 SenseVoice offline ASR 典型用法：
```rust
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig};

let config = OfflineRecognizerConfig {
    model: OfflineModelConfig {
        sense_voice: OfflineSenseVoiceModelConfig {
            model: "/path/to/model.int8.onnx".to_string(),
            language: "auto".to_string(),
            use_itn: true,
        },
        tokens: "/path/to/tokens.txt".to_string(),
        num_threads: 2,
        ..Default::default()
    },
    ..Default::default()
};
let recognizer = OfflineRecognizer::new(&config)?;
```

**Step 3: 确认动态库预构建下载链接**

macOS arm64: `https://github.com/k2-fsa/sherpa-onnx/releases/latest/download/sherpa-onnx-v<VER>-osx-arm64-shared.tar.bz2`
macOS x86: `https://github.com/k2-fsa/sherpa-onnx/releases/latest/download/sherpa-onnx-v<VER>-osx-x86_64-shared.tar.bz2`
Windows x64: `https://github.com/k2-fsa/sherpa-onnx/releases/latest/download/sherpa-onnx-v<VER>-win-x64-shared.tar.bz2`

**Step 4: Commit**

```bash
git commit -m "docs: add sensevoice-local integration plan"
```

---

## Task 2: Rust — 添加 sherpa-onnx 依赖 + build.rs 配置

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`（或修改现有）
- Create: `src-tauri/scripts/download_sherpa_libs.sh`（开发辅助脚本）

**Step 1: 添加依赖到 Cargo.toml**

```toml
[dependencies]
# ... 现有依赖 ...
sherpa-onnx = { version = "1", optional = true }

[features]
devtools = ["tauri/devtools"]
local-stt = ["dep:sherpa-onnx"]
```

**Step 2: 修改 build.rs 设置动态库搜索路径**

`src-tauri/build.rs` 现有内容只有 `tauri_build::build()`，扩展为：
```rust
fn main() {
    // 如果设置了 SHERPA_ONNX_LIB_DIR，告诉 rustc 去那里找动态库
    if let Ok(lib_dir) = std::env::var("SHERPA_ONNX_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    }
    tauri_build::build()
}
```

**Step 3: 创建开发辅助脚本**

`scripts/download_sherpa_libs.sh`：
```bash
#!/usr/bin/env bash
# 下载 sherpa-onnx 预构建共享库（macOS arm64）
# 用法: bash scripts/download_sherpa_libs.sh
set -e
VERSION="1.10.40"  # 更新时修改此处
PLATFORM="osx-arm64"
URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/v${VERSION}/sherpa-onnx-v${VERSION}-${PLATFORM}-shared.tar.bz2"
DEST="src-tauri/libs/sherpa-onnx"
mkdir -p "$DEST"
curl -L "$URL" | tar xj -C "$DEST" --strip-components=1
echo "Done. Set: export SHERPA_ONNX_LIB_DIR=$(pwd)/$DEST/lib"
```

**Step 4: 运行验证**

```bash
bash scripts/download_sherpa_libs.sh
export SHERPA_ONNX_LIB_DIR=$(pwd)/src-tauri/libs/sherpa-onnx/lib
cargo check --manifest-path src-tauri/Cargo.toml --features local-stt
```

Expected: `Finished` without errors（sherpa-onnx 是可选 feature，不强制所有环境）

**Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/build.rs scripts/download_sherpa_libs.sh
git commit -m "build: add sherpa-onnx optional dependency and lib setup script"
```

---

## Task 3: Rust — 实现 `SenseVoiceLocalProvider`

**Files:**
- Create: `src-tauri/src/stt/sensevoice_local.rs`
- Modify: `src-tauri/src/stt/mod.rs`

**Step 1: 创建 `sensevoice_local.rs`**

```rust
//! 本地离线 SenseVoice Small 推理 provider
//! 依赖: sherpa-onnx (optional feature "local-stt")

use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

use super::{SttConfig, SttProvider, TranscriptEvent};

/// 模型文件路径集合（由 model_dir 派生）
struct ModelPaths {
    model_onnx: PathBuf,
    tokens: PathBuf,
}

impl ModelPaths {
    fn from_dir(dir: &PathBuf) -> Self {
        Self {
            model_onnx: dir.join("model.int8.onnx"),
            tokens: dir.join("tokens.txt"),
        }
    }

    fn all_exist(&self) -> bool {
        self.model_onnx.exists() && self.tokens.exists()
    }
}

pub struct SenseVoiceLocalProvider {
    model_dir: PathBuf,
    audio_buffer: Vec<u8>,
    stt_config: Option<SttConfig>,
}

impl SenseVoiceLocalProvider {
    pub fn new(model_dir: PathBuf) -> Self {
        Self {
            model_dir,
            audio_buffer: Vec::new(),
            stt_config: None,
        }
    }
}

#[async_trait]
impl SttProvider for SenseVoiceLocalProvider {
    async fn connect(&mut self, config: &SttConfig) -> Result<()> {
        let paths = ModelPaths::from_dir(&self.model_dir);
        if !paths.all_exist() {
            anyhow::bail!(
                "SenseVoice model not found at {}. Please download it in Settings → Speech Recognition.",
                self.model_dir.display()
            );
        }
        self.stt_config = Some(config.clone());
        self.audio_buffer.clear();
        tracing::info!("SenseVoice Local provider ready (model: {})", self.model_dir.display());
        Ok(())
    }

    async fn send_audio(&mut self, chunk: &[u8]) -> Result<()> {
        self.audio_buffer.extend_from_slice(chunk);
        Ok(())
    }

    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>> {
        Ok(None) // 文件型 provider，在 disconnect() 做推理
    }

    async fn disconnect(&mut self) -> Result<Option<String>> {
        #[cfg(feature = "local-stt")]
        {
            use sherpa_onnx::{
                OfflineRecognizer, OfflineRecognizerConfig,
                OfflineModelConfig, OfflineSenseVoiceModelConfig,
                OfflineStream,
            };

            let config = match &self.stt_config {
                Some(c) => c.clone(),
                None => return Ok(None),
            };

            if self.audio_buffer.is_empty() {
                return Ok(None);
            }

            let paths = ModelPaths::from_dir(&self.model_dir);
            let sample_rate = config.sample_rate;

            // PCM i16 → f32 samples
            let samples: Vec<f32> = self.audio_buffer
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
                .collect();

            self.audio_buffer.clear();

            // 在 spawn_blocking 中运行 CPU 推理，避免阻塞 async executor
            let model_path = paths.model_onnx.to_string_lossy().into_owned();
            let tokens_path = paths.tokens.to_string_lossy().into_owned();
            let language = config.language.clone().unwrap_or_else(|| "auto".to_string());

            let text = tokio::task::spawn_blocking(move || -> Result<String> {
                let recognizer_config = OfflineRecognizerConfig {
                    model: OfflineModelConfig {
                        sense_voice: OfflineSenseVoiceModelConfig {
                            model: model_path,
                            language,
                            use_itn: true,
                        },
                        tokens: tokens_path,
                        num_threads: 2,
                        debug: 0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let recognizer = OfflineRecognizer::new(&recognizer_config)?;
                let mut stream = recognizer.create_stream();
                stream.accept_waveform(sample_rate, &samples);
                recognizer.decode_stream(&mut stream);
                let result = recognizer.get_result(&stream);
                Ok(result.text)
            })
            .await??;

            tracing::info!("SenseVoice Local transcription: {} chars", text.len());

            if text.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.trim().to_string()))
            }
        }

        #[cfg(not(feature = "local-stt"))]
        {
            anyhow::bail!("SenseVoice local feature not enabled in this build")
        }
    }

    fn name(&self) -> &str {
        "SenseVoice Small (Local)"
    }
}
```

**Step 2: 注册到 `stt/mod.rs`**

在 `mod.rs` 顶部添加：
```rust
#[cfg(feature = "local-stt")]
pub mod sensevoice_local;
```

在 `create_provider` 函数中添加（`match provider_name` 里）：
```rust
"sensevoice-local" => {
    let model_dir = crate::get_sensevoice_model_dir(); // 见 Task 5
    Box::new(sensevoice_local::SenseVoiceLocalProvider::new(model_dir))
}
```

对非 `local-stt` feature 构建，fallback 到错误提示：
```rust
#[cfg(not(feature = "local-stt"))]
"sensevoice-local" => {
    make(WhisperCompatConfig {
        provider_name: "SenseVoice (Local — not built)",
        endpoint: "http://localhost:1/",
        model: "",
        extra_fields: &[],
    })
}
```

**Step 3: Cargo 编译验证**

```bash
cargo check --manifest-path src-tauri/Cargo.toml --features local-stt
```

Expected: no errors

**Step 4: Commit**

```bash
git add src-tauri/src/stt/sensevoice_local.rs src-tauri/src/stt/mod.rs
git commit -m "feat(stt): add SenseVoiceLocalProvider skeleton"
```

---

## Task 4: Rust — 模型管理 IPC 命令

模型下载需要：进度上报、断点、取消、校验完整性。

**Files:**
- Create: `src-tauri/src/model_manager.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 `model_manager.rs`**

```rust
//! 本地 STT 模型下载与管理

use anyhow::Result;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// SenseVoice Small 模型文件下载 URL（int8 量化版）
const MODEL_FILES: &[(&str, &str)] = &[
    (
        "model.int8.onnx",
        "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2",
    ),
    // tokens.txt 包含在同一个 tarball 中，解压即可
];

/// 返回 SenseVoice Small 模型存储目录
pub fn sensevoice_model_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app_data_dir unavailable")
        .join("models")
        .join("sensevoice-small")
}

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub is_downloaded: bool,
    pub model_dir: String,
    pub size_mb: Option<f64>,
}

/// 查询模型是否已下载
#[tauri::command]
pub async fn get_sensevoice_model_status(app: AppHandle) -> ModelStatus {
    let dir = sensevoice_model_dir(&app);
    let model_file = dir.join("model.int8.onnx");
    let tokens_file = dir.join("tokens.txt");
    let is_downloaded = model_file.exists() && tokens_file.exists();

    let size_mb = if is_downloaded {
        std::fs::metadata(&model_file)
            .ok()
            .map(|m| m.len() as f64 / 1_000_000.0)
    } else {
        None
    };

    ModelStatus {
        is_downloaded,
        model_dir: dir.to_string_lossy().into_owned(),
        size_mb,
    }
}

/// 下载 SenseVoice Small 模型（带进度事件）
/// 发射事件: `model:download-progress` { downloaded: u64, total: u64, percent: f32 }
/// 发射事件: `model:download-complete`
/// 发射事件: `model:download-error` { message: String }
#[tauri::command]
pub async fn download_sensevoice_model(app: AppHandle) -> Result<(), String> {
    let dir = sensevoice_model_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let tarball_url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2";
    let tmp_path = dir.join("download.tar.bz2");

    // 下载 tarball（带进度）
    let client = reqwest::Client::new();
    let resp = client
        .get(tarball_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| e.to_string())?;

    use tokio::io::AsyncWriteExt;
    use futures_util::StreamExt;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let percent = if total > 0 {
            downloaded as f32 / total as f32 * 100.0
        } else {
            0.0
        };
        let _ = app.emit(
            "model:download-progress",
            serde_json::json!({
                "downloaded": downloaded,
                "total": total,
                "percent": percent
            }),
        );
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    // 解压 tarball
    let tmp_path_clone = tmp_path.clone();
    let dir_clone = dir.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::open(&tmp_path_clone).map_err(|e| e.to_string())?;
        let decompressed = bzip2::read::BzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressed);
        // 只提取 model.int8.onnx 和 tokens.txt，跳过其余文件
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?;
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "model.int8.onnx" || name == "tokens.txt" {
                let dest = dir_clone.join(name);
                entry.unpack(&dest).map_err(|e| e.to_string())?;
            }
        }
        std::fs::remove_file(&tmp_path_clone).ok();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app.emit("model:download-complete", ());
    Ok(())
}

/// 删除已下载的模型文件（释放磁盘空间）
#[tauri::command]
pub async fn delete_sensevoice_model(app: AppHandle) -> Result<(), String> {
    let dir = sensevoice_model_dir(&app);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

注意：需要添加 `bzip2` 和 `tar` 依赖到 `Cargo.toml`：
```toml
bzip2 = "0.4"
tar = "0.4"
```

**Step 2: 在 `lib.rs` 注册命令**

找到 `.invoke_handler(tauri::generate_handler![...])` 处，添加：
```rust
model_manager::get_sensevoice_model_status,
model_manager::download_sensevoice_model,
model_manager::delete_sensevoice_model,
```

同时在文件顶部添加：
```rust
mod model_manager;
```

并添加供 `stt/mod.rs` 使用的辅助函数：
```rust
pub fn get_sensevoice_model_dir_from_path(data_dir: std::path::PathBuf) -> std::path::PathBuf {
    data_dir.join("models").join("sensevoice-small")
}
```

**Step 3: 修改 `pipeline.rs` 跳过 API key 校验**

第 312 行：
```rust
// 原来:
if config_data.stt_api_key.is_empty() && config_data.stt_provider != "cloud" {

// 修改为:
if config_data.stt_api_key.is_empty()
    && config_data.stt_provider != "cloud"
    && config_data.stt_provider != "sensevoice-local"
{
```

**Step 4: 编译验证**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors

**Step 5: Commit**

```bash
git add src-tauri/src/model_manager.rs src-tauri/src/lib.rs src-tauri/src/pipeline.rs src-tauri/Cargo.toml
git commit -m "feat(model): add model download/status IPC commands and pipeline bypass for local STT"
```

---

## Task 5: Rust — 修复 `create_provider` 获取 model_dir

`stt/mod.rs` 的 `create_provider` 目前没有 AppHandle，需要传入 model_dir。

**Files:**
- Modify: `src-tauri/src/stt/mod.rs`
- Modify: `src-tauri/src/pipeline.rs`

**Step 1: 修改 `create_provider` 签名**

```rust
pub fn create_provider(
    provider_name: &str,
    client: Option<reqwest::Client>,
    app_data_dir: Option<std::path::PathBuf>,  // 新增
) -> Box<dyn SttProvider> {
    // ...
    "sensevoice-local" => {
        let model_dir = app_data_dir
            .map(|d| d.join("models").join("sensevoice-small"))
            .unwrap_or_else(|| std::path::PathBuf::from("models/sensevoice-small"));
        Box::new(sensevoice_local::SenseVoiceLocalProvider::new(model_dir))
    }
```

**Step 2: 修改 `pipeline.rs` 调用处**

`pipeline.rs` 第 358 行：
```rust
// 原来:
let mut provider = stt::create_provider(&config_data.stt_provider, Some(self.shared_client.clone()));

// 修改为:
let app_data_dir = self.app_handle.path().app_data_dir().ok();
let mut provider = stt::create_provider(
    &config_data.stt_provider,
    Some(self.shared_client.clone()),
    app_data_dir,
);
```

同样修改 `bench_stt_connection` 命令中的调用（在 `lib.rs`）。

**Step 3: Commit**

```bash
git add src-tauri/src/stt/mod.rs src-tauri/src/pipeline.rs src-tauri/src/lib.rs
git commit -m "refactor(stt): pass app_data_dir to create_provider for local model path"
```

---

## Task 6: Frontend — Tauri IPC 封装

**Files:**
- Modify: `src/lib/tauri.ts`
- Create: `src/lib/modelManager.ts`

**Step 1: 在 `tauri.ts` 添加 model IPC 调用**

```typescript
export async function getSenseVoiceModelStatus(): Promise<{
  isDownloaded: boolean
  modelDir: string
  sizeMb: number | null
}> {
  return invoke('get_sensevoice_model_status')
}

export async function downloadSenseVoiceModel(): Promise<void> {
  return invoke('download_sensevoice_model')
}

export async function deleteSenseVoiceModel(): Promise<void> {
  return invoke('delete_sensevoice_model')
}
```

**Step 2: 在 `tauri.ts` 添加 model download 事件监听工具**

```typescript
import { listen } from '@tauri-apps/api/event'

export type ModelDownloadProgress = {
  downloaded: number
  total: number
  percent: number
}

export function onModelDownloadProgress(
  cb: (p: ModelDownloadProgress) => void,
): () => void {
  let unlisten: (() => void) | null = null
  listen<ModelDownloadProgress>('model:download-progress', (e) => cb(e.payload)).then(
    (fn) => (unlisten = fn),
  )
  return () => unlisten?.()
}

export function onModelDownloadComplete(cb: () => void): () => void {
  let unlisten: (() => void) | null = null
  listen('model:download-complete', () => cb()).then((fn) => (unlisten = fn))
  return () => unlisten?.()
}
```

**Step 3: Commit**

```bash
git add src/lib/tauri.ts
git commit -m "feat(ipc): add SenseVoice model management IPC wrappers"
```

---

## Task 7: Frontend — `SttPane.tsx` 模型管理 UI

当 provider 选择 `sensevoice-local` 时，显示模型管理面板（类似截图中的样式）。

**Files:**
- Modify: `src/components/Settings/SttPane.tsx`
- Modify: `src/lib/constants.ts`
- Modify: `src/i18n/locales/zh.json`（及其他语言文件）

**Step 1: 更新 constants.ts，添加 provider**

```typescript
export const STT_PROVIDERS = [
  { value: 'sensevoice-local', label: 'SenseVoice Small (本地 · 免费)' },  // 新增，放在首位
  { value: 'deepgram', label: 'Deepgram Nova-3' },
  // ... 其余不变
] as const
```

**Step 2: 在 `SttPane.tsx` 添加 `SenseVoiceLocalPanel` 子组件**

```tsx
import { useState, useEffect } from 'react'
import { Download, Trash2, FolderOpen, CheckCircle2, Loader2 } from 'lucide-react'
import {
  getSenseVoiceModelStatus,
  downloadSenseVoiceModel,
  deleteSenseVoiceModel,
  onModelDownloadProgress,
  onModelDownloadComplete,
} from '../../lib/tauri'

type ModelStatus = {
  isDownloaded: boolean
  modelDir: string
  sizeMb: number | null
}

function SenseVoiceLocalPanel() {
  const { t } = useTranslation()
  const [status, setStatus] = useState<ModelStatus | null>(null)
  const [downloading, setDownloading] = useState(false)
  const [progress, setProgress] = useState(0) // 0-100

  const refresh = async () => {
    const s = await getSenseVoiceModelStatus()
    setStatus(s)
  }

  useEffect(() => {
    refresh()
  }, [])

  useEffect(() => {
    const unsub1 = onModelDownloadProgress((p) => setProgress(p.percent))
    const unsub2 = onModelDownloadComplete(() => {
      setDownloading(false)
      setProgress(100)
      refresh()
    })
    return () => { unsub1(); unsub2() }
  }, [])

  const handleDownload = async () => {
    setDownloading(true)
    setProgress(0)
    try {
      await downloadSenseVoiceModel()
    } catch (e) {
      setDownloading(false)
    }
  }

  const handleDelete = async () => {
    if (!confirm(t('settings.senseVoiceDeleteConfirm'))) return
    await deleteSenseVoiceModel()
    refresh()
  }

  return (
    <div className="border border-border rounded-[10px] p-4 space-y-3">
      {/* 本地模型标题 */}
      <div className="flex items-center gap-2">
        <HardDrive size={14} className="text-text-secondary" />
        <span className="text-[13px] font-medium text-text-primary">{t('settings.localModel')}</span>
        <span className="text-[11px] text-text-tertiary">{t('settings.localModelHint')}</span>
      </div>

      {status?.isDownloaded ? (
        <div className="space-y-2">
          <div className="flex items-center gap-1.5 text-[12px] text-success">
            <CheckCircle2 size={13} />
            <span>{t('settings.modelReady')}</span>
            {status.sizeMb && <span className="text-text-tertiary">({status.sizeMb.toFixed(0)} MB)</span>}
          </div>
          <button
            onClick={handleDelete}
            className="flex items-center gap-1.5 text-[12px] text-error hover:underline"
          >
            <Trash2 size={12} />
            {t('settings.deleteModel')}
          </button>
        </div>
      ) : downloading ? (
        <div className="space-y-1.5">
          <div className="flex items-center gap-2 text-[12px] text-text-secondary">
            <Loader2 size={13} className="animate-spin" />
            <span>{t('settings.downloading')} {progress.toFixed(0)}%</span>
          </div>
          <div className="w-full h-1.5 bg-bg-secondary rounded-full overflow-hidden">
            <div
              className="h-full bg-accent transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      ) : (
        <div className="space-y-2">
          <p className="text-[12px] text-text-secondary">{t('settings.modelNotDownloaded')}</p>
          <button
            onClick={handleDownload}
            className="flex items-center gap-1.5 px-3 py-2 bg-accent text-white rounded-[8px] text-[12px] hover:bg-accent-hover transition-colors"
          >
            <Download size={13} />
            {t('settings.downloadModel')} (~360 MB)
          </button>
        </div>
      )}
    </div>
  )
}
```

**Step 3: 在 `SttPane` 渲染逻辑中使用**

在现有的 `isCloud` 判断之后添加：
```tsx
const isLocalSenseVoice = config.stt_provider === 'sensevoice-local'

// 在 JSX 中:
{isLocalSenseVoice ? (
  <SenseVoiceLocalPanel />
) : isCloud ? (
  // ... 现有 Cloud UI
) : (
  // ... 现有 API Key UI
)}
```

**Step 4: 添加 i18n 翻译（zh.json）**

```json
"settings": {
  ...
  "localModel": "本地模型",
  "localModelHint": "运行在您的设备上，无需联网",
  "modelReady": "模型已就绪，可以使用",
  "deleteModel": "删除模型文件",
  "modelNotDownloaded": "首次使用需下载模型文件（约 360 MB）",
  "downloadModel": "下载模型",
  "downloading": "下载中",
  "senseVoiceDeleteConfirm": "确定删除模型文件？删除后需重新下载才能使用本地识别。"
}
```

同样更新 `en.json` 等其他语言文件。

**Step 5: TypeScript 类型检查**

```bash
npx tsc --noEmit
```

Expected: no errors

**Step 6: Commit**

```bash
git add src/components/Settings/SttPane.tsx src/lib/constants.ts src/i18n/locales/
git commit -m "feat(ui): add SenseVoice Local model management panel in STT settings"
```

---

## Task 8: 本地集成测试

**Step 1: 下载真实模型文件进行端到端测试**

```bash
bash scripts/download_sherpa_libs.sh       # 下载共享库
# 然后在 app data dir 手动放置模型文件（或通过 UI 下载）
npm run tauri dev
```

**Step 2: 验证关键路径**

- [ ] 选择 "SenseVoice Small (本地 · 免费)" provider → 不显示 API key 输入框
- [ ] 模型未下载时显示下载按钮
- [ ] 点击下载 → 进度条正确更新
- [ ] 下载完成 → 显示"模型已就绪"
- [ ] 录音 → 转录 → 结果输出（端到端）
- [ ] 删除模型 → 重新变为"未下载"状态

**Step 3: 验证无 local-stt feature 的构建不受影响**

```bash
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: 正常编译，sensevoice-local provider 存在但 connect() 会返回 feature not enabled 错误

**Step 4: Commit**

```bash
git commit -m "test: verify SenseVoice local end-to-end flow"
```

---

## Task 9: CI / 构建配置

**Step 1: 更新 `.github/workflows/` 的 macOS 构建步骤**

在 macOS CI job 中添加：
```yaml
- name: Download sherpa-onnx shared libs (macOS)
  run: bash scripts/download_sherpa_libs.sh
  env:
    SHERPA_ONNX_LIB_DIR: ${{ github.workspace }}/src-tauri/libs/sherpa-onnx/lib

- name: Build Tauri (with local-stt feature)
  run: npm run tauri build -- --features local-stt
  env:
    SHERPA_ONNX_LIB_DIR: ${{ github.workspace }}/src-tauri/libs/sherpa-onnx/lib
```

**Step 2: 更新 `tauri.conf.json` bundle 配置**

将 sherpa-onnx 共享库打包进 app bundle（macOS .app）：
```json
{
  "bundle": {
    "resources": {
      "src-tauri/libs/sherpa-onnx/lib/*.dylib": "libs/"
    }
  }
}
```

**Step 3: Commit**

```bash
git add .github/ src-tauri/tauri.conf.json
git commit -m "ci: add sherpa-onnx build support for local STT feature"
```

---

## 实施顺序总结

| 任务 | 预计时长 | 关键风险 |
|---|---|---|
| Task 1: 调研确认 API | 2h | sherpa-onnx Rust API 可能有版本差异，需要核对最新版 |
| Task 2: 依赖配置 | 1h | 动态库链接在不同平台路径不同 |
| Task 3: Rust provider | 3h | PCM → f32 转换格式需精确，sherpa-onnx API 精确签名需核对 |
| Task 4: IPC 命令 | 2h | bzip2 tarball 解压路径结构需确认 |
| Task 5: create_provider 重构 | 1h | 需要更新所有调用点 |
| Task 6: Frontend IPC 封装 | 1h | 低风险 |
| Task 7: UI 组件 | 3h | 状态管理 + i18n，工作量较大 |
| Task 8: 端到端测试 | 2h | 需要真实设备测试推理速度 |
| Task 9: CI 配置 | 1h | 各平台库路径差异 |

**总计：约 16 小时**

---

## 已知风险和缓解措施

1. **sherpa-onnx Rust crate 版本锁定**：sherpa-onnx 更新较频繁，需要在 Cargo.toml 锁定精确版本，避免 API 变化
2. **下载源稳定性**：GitHub releases 在中国大陆访问不稳定，未来可考虑提供镜像 CDN
3. **推理速度**：低端机 CPU 推理可能 3-8 秒，需要在 UI 上给用户合理预期
4. **共享库签名（macOS）**：macOS 应用公证需要对 dylib 进行代码签名，CI 配置复杂度增加
5. **模型 tarball 内部路径**：需实际下载 tarball 确认 `model.int8.onnx` 和 `tokens.txt` 的内部路径

---

## 后续优化（本次范围外）

- 首次启动自动检测 → 弹窗提示下载（onboarding 体验）
- 支持 GPU 加速（Metal on macOS）
- 支持其他本地模型（Whisper tiny、Paraformer 等）
- 下载时支持取消
- 镜像下载源（国内 CDN）
