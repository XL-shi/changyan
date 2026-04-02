use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

use super::{SttConfig, SttProvider, TranscriptEvent};

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
        let model_file = self.model_dir.join("model.int8.onnx");
        let tokens_file = self.model_dir.join("tokens.txt");
        if !model_file.exists() || !tokens_file.exists() {
            anyhow::bail!(
                "SenseVoice model not found at {}. Please download it in Settings → Speech Recognition.",
                self.model_dir.display()
            );
        }
        self.stt_config = Some(config.clone());
        self.audio_buffer.clear();
        tracing::info!(
            "SenseVoice Local provider ready (model: {})",
            self.model_dir.display()
        );
        Ok(())
    }

    async fn send_audio(&mut self, chunk: &[u8]) -> Result<()> {
        self.audio_buffer.extend_from_slice(chunk);
        Ok(())
    }

    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>> {
        Ok(None)
    }

    async fn disconnect(&mut self) -> Result<Option<String>> {
        let config = match self.stt_config.take() {
            Some(c) => c,
            None => return Ok(None),
        };

        if self.audio_buffer.is_empty() {
            return Ok(None);
        }

        let audio_bytes = std::mem::take(&mut self.audio_buffer);
        let sample_rate = config.sample_rate;
        let model_path = self
            .model_dir
            .join("model.int8.onnx")
            .to_string_lossy()
            .into_owned();
        let tokens_path = self
            .model_dir
            .join("tokens.txt")
            .to_string_lossy()
            .into_owned();
        let language = config
            .language
            .clone()
            .unwrap_or_else(|| "auto".to_string());

        let text = tokio::task::spawn_blocking(move || -> Result<String> {
            #[cfg(feature = "local-stt")]
            {
                use sherpa_onnx::{
                    OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
                };

                // PCM i16 → f32
                let samples: Vec<f32> = audio_bytes
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
                    .collect();

                let mut cfg = OfflineRecognizerConfig::default();
                cfg.model_config.sense_voice = OfflineSenseVoiceModelConfig {
                    model: Some(model_path),
                    language: Some(language),
                    use_itn: true,
                };
                cfg.model_config.tokens = Some(tokens_path);
                cfg.model_config.num_threads = 2;

                let recognizer = OfflineRecognizer::create(&cfg)
                    .ok_or_else(|| anyhow::anyhow!("Failed to create SenseVoice recognizer"))?;
                let stream = recognizer.create_stream();
                stream.accept_waveform(sample_rate as i32, &samples);
                recognizer.decode(&stream);
                let result = stream
                    .get_result()
                    .ok_or_else(|| anyhow::anyhow!("No result from SenseVoice"))?;
                Ok(result.text)
            }
            #[cfg(not(feature = "local-stt"))]
            {
                let _ = (audio_bytes, sample_rate, model_path, tokens_path, language);
                anyhow::bail!("SenseVoice local feature not enabled in this build")
            }
        })
        .await??;

        let text = text.trim().to_string();
        tracing::info!("SenseVoice Local transcription: {} chars", text.len());

        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    fn name(&self) -> &str {
        "SenseVoice Small (Local)"
    }
}
