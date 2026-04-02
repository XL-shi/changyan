use anyhow::Result;
use futures_util::StreamExt;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

const MODEL_TARBALL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17.tar.bz2";
const _TARBALL_DIR_PREFIX: &str = "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17";

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

#[tauri::command]
pub async fn download_sensevoice_model(app: AppHandle) -> Result<(), String> {
    let dir = sensevoice_model_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let tmp_path = dir.join("download.tar.bz2");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(MODEL_TARBALL_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| e.to_string())?;

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

    let tmp_path_clone = tmp_path.clone();
    let dir_clone = dir.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::open(&tmp_path_clone).map_err(|e| e.to_string())?;
        let decompressed = bzip2::read::BzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressed);

        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?;
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if file_name == "model.int8.onnx" || file_name == "tokens.txt" {
                let dest = dir_clone.join(&file_name);
                entry.unpack(&dest).map_err(|e| e.to_string())?;
                tracing::info!("Extracted: {}", file_name);
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

#[tauri::command]
pub async fn delete_sensevoice_model(app: AppHandle) -> Result<(), String> {
    let dir = sensevoice_model_dir(&app);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
