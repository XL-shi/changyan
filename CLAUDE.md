# CLAUDE.md

## Reasoning & Interaction Protocol

运用第一性原理思考，拒绝经验主义和路径盲从。不假设用户完全清楚目标，保持审慎，从原始需求和问题出发。若目标模糊，停下来讨论；若目标清晰但路径非最优，直接建议更短、更低成本的办法。

所有回答分为两个部分：

- **直接执行**：按照用户当前的要求和逻辑，直接给出任务结果。
- **深度交互**：仅当判断路径可能非最优、存在 XY 问题、或有明显更优替代方案时，才主动提出挑战——分析当前路径的弊端并给出更优雅的替代方案。任务简单明确时可省略此部分。

## Project Overview

**ChangYan** — AI-powered speech-to-text desktop app built with Tauri 2 (React + Rust).

Core flow: Microphone → STT provider → LLM polish → keyboard/clipboard output.

## Tech Stack

- **Frontend**: React 19, TypeScript, Zustand, Vite, Tailwind CSS 4, Framer Motion, i18next
- **Backend**: Rust (Tauri 2), SQLite (rusqlite), cpal (audio), enigo (keyboard sim)
- **STT providers**: Deepgram, AssemblyAI, Whisper, Groq, GLM-ASR, SiliconFlow, Cloud
- **LLM providers**: OpenAI, DeepSeek, Claude, Gemini, Ollama, Groq, Moonshot, Qwen, OpenRouter, Zhipu, Cloud

## Common Commands

```bash
# Development
npm run dev           # Start Vite dev server (port 1420)
npm run tauri dev     # Start full Tauri app (frontend + Rust backend)

# Build
npm run tauri build         # Production build for current platform
npm run tauri:build:signed  # Build + ad-hoc sign (macOS only)

# Manual signing (macOS)
./scripts/adhoc-sign.sh /path/to/ChangYan.app
./scripts/build-and-sign.sh  # Build, sign, and create DMG

# Lint & Format (run before committing)
npx tsc --noEmit                          # TypeScript check
npx eslint src/                           # ESLint
npx prettier --check src/                 # Prettier check
npx prettier --write src/                 # Prettier fix

# Tests
npm test              # Run Vitest (frontend)
cargo test --manifest-path src-tauri/Cargo.toml  # Rust tests

# Rust checks
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Architecture

### Two Windows

- **Main window** (900×700): Settings, History, Home, Account pages
- **Capsule window** (80×80): Floating recording widget, always-on-top, transparent

### Frontend Structure (`src/`)

- `components/` — UI components (Capsule states, Settings panes, History, etc.)
- `stores/` — Zustand state (`appStore.ts` for config/pipeline, `authStore.ts` for auth)
- `hooks/` — React hooks (recording, Tauri events, theme)
- `lib/` — Tauri IPC wrapper (`tauri.ts`), cloud API client (`api.ts`), auth client
- `i18n/locales/` — Translation files for 18+ languages

### Backend Structure (`src-tauri/src/`)

- `lib.rs` — App setup, IPC command registration, hotkey, system tray
- `pipeline.rs` — Recording → STT → LLM → output orchestration
- `audio/` — cpal audio capture
- `stt/` — STT provider integrations
- `llm/` — LLM provider integrations (supports streaming)
- `output/` — keyboard.rs (enigo), clipboard.rs
- `storage/` — Config (tauri-plugin-store), SQLite (history + dictionary)

### State Flow

1. Global hotkey triggers recording start/stop
2. Rust emits `pipeline:state-changed` events to frontend (Idle → Recording → Processing → Polishing → Complete)
3. Final text output via keyboard simulation or clipboard paste

## Code Style

- **TypeScript**: Strict mode, no unused vars (prefix `_` to suppress), no semicolons, single quotes, 100-char line width
- **Rust**: `cargo fmt` + `cargo clippy -D warnings` must pass
- **Commits**: [Conventional Commits](https://www.conventionalcommits.org/) format
- **Tailwind**: Utility-first, no custom CSS unless necessary

## Key Config

App config stored via `tauri-plugin-store` (JSON). Key fields:

- `stt_provider`, `stt_api_key`, `stt_language`
- `llm_provider`, `llm_api_key`, `llm_model`, `llm_base_url`
- `polish_enabled`, `output_mode` (`keyboard` | `clipboard`)
- `hotkey`, `hotkey_mode` (`hold` | `toggle`)

## Database

SQLite (`opentypeless.db`) via rusqlite — migrations in `src-tauri/migrations/`:

- `history` table — transcription records with timestamps, raw/polished text, app context
- `dictionary` table — user-defined terms for transcription accuracy

## Environment Variables

- `VITE_API_BASE_URL` — Cloud API base (default: `https://www.opentypeless.com`)
- `API_BASE_URL` — Rust-side cloud API base

## Testing

- **Frontend**: Vitest + jsdom + `@testing-library/react`; tests in `src/**/__tests__/`
- **Backend**: `cargo test`
- **CI**: GitHub Actions runs TypeScript check, ESLint, Prettier, Vitest, Cargo fmt/clippy/test on Windows/macOS/Linux

## Important Notes

- The capsule window is a separate Tauri window (`#capsule` hash route), not a React route
- API keys are stored locally and never sent to the ChangYan cloud in BYOK mode
- macOS: keys encrypted via Keychain through tauri-plugin-store
- Deep link auth flow: `changyan://callback?token=...`
- Always check that both frontend and Rust compile cleanly before submitting changes
