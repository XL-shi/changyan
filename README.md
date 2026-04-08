# ChangYan

ChangYan is a Tauri 2 desktop app for speech-to-text and AI text polishing.
Press a hotkey, speak naturally, and ChangYan transcribes, rewrites, and outputs text directly into the app you're using.

[Releases](https://github.com/XL-shi/changyan/releases) | [Issues](https://github.com/XL-shi/changyan/issues) | [Repository](https://github.com/XL-shi/changyan)

---

## Why ChangYan?

| | ChangYan | macOS Dictation | Windows Voice Typing | Whisper Desktop |
|---|---|---|---|---|
| AI text polishing | ✅ Multiple LLMs | ❌ | ❌ | ❌ |
| STT flexibility | ✅ Local, direct API, or cloud | ❌ Apple only | ❌ Microsoft only | ❌ Whisper only |
| Works in any app | ✅ | ✅ | ✅ | ❌ Copy-paste |
| Translation mode | ✅ | ❌ | ❌ | ❌ |
| Local STT option | ✅ SenseVoice Small | ❌ | ❌ | ✅ |
| Cross-platform | ✅ Win/Mac/Linux | ❌ Mac only | ❌ Windows only | ✅ |
| Custom dictionary | ✅ | ❌ | ❌ | ❌ |
| Self-hostable | ✅ BYOK | ❌ | ❌ | ✅ |

## Features

- 🎙️ Global hotkey recording with hold-to-record or toggle mode
- 💊 Floating capsule window for recording and processing status
- 🗣️ Multiple STT options: local `sensevoice-local`, Deepgram, AssemblyAI, GLM-ASR, OpenAI Whisper, Groq Whisper, SiliconFlow, and ChangYan Cloud
- 🤖 Multiple LLM providers: Zhipu, DeepSeek, SiliconFlow, OpenAI, Gemini, Moonshot, Qwen, Groq, Claude, Ollama, OpenRouter, and ChangYan Cloud
- ⚡ Streaming output — text appears as the LLM generates it
- ⌨️ Keyboard simulation or clipboard output
- 📝 Highlight text before recording to give the LLM context
- 🌐 Translation mode with 20+ target languages
- 📖 Custom dictionary for domain-specific terms
- 🔍 Per-app context detection for better prompting and formatting
- 📜 Local history with full-text search
- 🌗 Dark / light / system theme
- 🚀 Auto-start on login

> [!TIP]
> **Recommended Configuration for Best Experience**
>
> | | Provider | Model |
> |---|---|---|
> | 🗣️ STT | Local | `sensevoice-small` |
> | 🤖 AI Polish | DeepSeek | `deepseek-chat` |
>
> This setup gives you a low-latency local transcription path plus strong rewriting quality for daily use.

## Download & Installation

### macOS

#### Option 1: Homebrew (Recommended) ⭐

The easiest way to install on macOS — no manual authorization needed:

```bash
brew tap XL-shi/changyan
brew install --cask changyan
```

**Advantages**:
- ✅ One-line installation
- ✅ Automatic updates with `brew upgrade`
- ✅ No signing issues — Homebrew handles everything
- ✅ Easy uninstall with `brew uninstall --cask changyan`

#### Option 2: Manual DMG Install

**[Download from Releases](https://github.com/XL-shi/changyan/releases)**

| Architecture | File |
|--------------|------|
| Apple Silicon (M1/M2/M3) | `ChangYan_*_aarch64.dmg` |
| Intel | `ChangYan_*_x86_64.dmg` |

**⚠️ Important**: The app uses ad-hoc signing (free, no Apple Developer account). First launch requires manual authorization:

**Method 1 (Easiest)** - Terminal command:
```bash
xattr -cr /Applications/ChangYan.app
```

**Method 2** - Right-click open:
1. Drag `ChangYan.app` to Applications folder
2. Right-click → "Open" → Click "Open" in dialog
3. Only needed once, then opens normally

**Method 3** - System Settings:
1. Try to open the app (error will show)
2. Open System Settings → Privacy & Security
3. Click "Open Anyway" under Security section

> **Why?** macOS Gatekeeper blocks apps without Apple Developer ID ($99/year). The ad-hoc signature ensures the app hasn't been tampered with, but requires one-time manual authorization. **Homebrew automates this for you.**

### Windows

**[Download from Releases](https://github.com/XL-shi/changyan/releases)**

Download and run the `.msi` installer.

### Linux

**[Download from Releases](https://github.com/XL-shi/changyan/releases)**

- **Debian/Ubuntu**: `sudo dpkg -i changyan_*.deb`
- **Fedora/RHEL**: `sudo rpm -i changyan-*.rpm`
- **AppImage**: `chmod +x ChangYan-*.AppImage && ./ChangYan-*.AppImage`

## Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable toolchain)
- Platform-specific dependencies for Tauri: see [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

## Getting Started

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Configuration

All settings are accessible from the in-app Settings panel:

- **Speech Recognition** — choose STT provider and enter your API key
- **AI Polish** — choose LLM provider, model, and API key
- **General** — hotkey, output mode, theme, auto-start
- **Dictionary** — add custom terms for better transcription accuracy
- **Scenes** — prompt templates for different use cases

API keys are stored locally via `tauri-plugin-store`. In BYOK mode, STT and LLM requests go directly to the provider you configure.

### Cloud Option

ChangYan also supports an optional cloud mode for managed STT / LLM access, backup, restore, and account-based features. This is optional — the app remains usable with your own keys or local models.

### BYOK (Bring Your Own Key) vs Cloud

| | BYOK Mode | Cloud Mode |
|---|---|---|
| STT | Your own API key or local model | Managed STT quota |
| LLM | Your own API key or local model | Managed LLM quota |
| Cloud dependency | None for direct-provider mode | Requires a configured cloud backend |
| Account features | Not required | Sign-in, backup, restore, scene packs |

All core features — recording, transcription, AI polish, keyboard/clipboard output, dictionary, and history — work without ChangYan Cloud when you use direct providers or local models.

### Self-Hosting / No Cloud

To run ChangYan without any cloud dependency:

1. Choose any non-Cloud STT and LLM provider in Settings
2. Use local models or enter your own API keys
3. That's it — no ChangYan account is required

If you want to point optional cloud features at your own backend, set these environment variables before building:

| Variable | Description |
|---|---|
| `VITE_API_BASE_URL` | Frontend cloud API base URL |
| `API_BASE_URL` | Rust backend cloud API base URL |

```bash
# Example: build with a custom backend
VITE_API_BASE_URL=https://my-server.example.com API_BASE_URL=https://my-server.example.com npm run tauri build
```

## Architecture

**Data Flow Pipeline:**

```text
Microphone → Audio Capture → STT Provider → Raw Transcript → LLM Polish → Keyboard/Clipboard Output
```

```text
src/                  # React frontend (TypeScript)
├── components/       # UI components (Settings, History, Capsule, etc.)
├── hooks/            # React hooks (recording, theme, Tauri events)
├── lib/              # Utilities (API client, router, constants)
└── stores/           # Zustand state management

src-tauri/src/        # Rust backend
├── audio/            # Audio capture via cpal
├── stt/              # STT providers (local, direct API, and cloud)
├── llm/              # LLM providers (direct API and cloud)
├── output/           # Text output (keyboard simulation, clipboard paste)
├── storage/          # Config (tauri-plugin-store) + history/dictionary (SQLite)
├── app_detector/     # Detect active application for context
├── pipeline.rs       # Recording → STT → LLM → Output orchestration
└── lib.rs            # Tauri app setup, commands, hotkey handling
```

## FAQ

**Is my audio sent to the cloud?**
In BYOK mode, audio goes directly to your chosen STT provider or local model. In cloud mode, audio is sent to the configured cloud backend.

**Can I use it offline?**
With a local STT provider such as SenseVoice Small and a local LLM such as Ollama, the app can work without external cloud services.

**Which languages are supported?**
STT supports 99+ languages depending on the provider. AI polish and translation support 20+ target languages.

**Is the app free?**
The app is fully functional with your own API keys or local models. Cloud-based features are optional.

## Project Links

- 🐛 [Issue Tracker](https://github.com/XL-shi/changyan/issues) — Bug reports and feature requests
- 📖 [Contributing Guide](CONTRIBUTING.md) — Development setup and guidelines
- 🔒 [Security Policy](SECURITY.md) — Report vulnerabilities responsibly
- 🧭 [Vision](VISION.md) — Project principles and roadmap direction
- 💬 [Support](SUPPORT.md) — How to get help

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

[MIT](LICENSE)

## 启动
平时完整开发：npm run tauri:dev
测 onboarding / 模型下载隔离环境：npm run tauri:dev:devtest
只改前端页面：npm run dev

## 发布版本
同步提高 package.json、src-tauri/Cargo.toml、src-tauri/tauri.conf.json 里的 version