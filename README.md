# 畅言

**AI 语音输入桌面工具** — 按下快捷键，开口说话，文字直接打进任何应用。

[下载](https://github.com/XL-shi/changyan/releases) · [反馈 / Issues](https://github.com/XL-shi/changyan/issues)

---

## 核心功能

- **一键语音输入** — 全局快捷键，按下开始说话，松开即输出
- **本地转写，免费离线** — 内置 SenseVoice Small，无需 API Key，首次使用自动下载
- **AI 润色** — 接入自己的大模型 Key，原始转写稿自动变成通顺的书面表达
- **输出到任意应用** — 模拟键盘输入或写入剪贴板，兼容所有输入框
- **翻译模式** — 说中文，输出英文（或其他 20+ 语言）
- **历史记录** — 本地保存所有转写结果，支持全文搜索
- **自定义词典** — 添加专有名词，提升领域识别准确率
- **深色 / 浅色主题**，开机自启

---

## 支持的服务商

| 类型 | 支持的服务商 |
|------|-------------|
| 语音识别 (STT) | SenseVoice Small（本地免费）、Deepgram、AssemblyAI、智谱 GLM-ASR、OpenAI Whisper、Groq Whisper、硅基流动 |
| AI 润色 (LLM) | 智谱、DeepSeek、硅基流动、OpenAI、Gemini、Moonshot (Kimi)、通义千问、Groq、Claude、Ollama（本地）、OpenRouter |

> **推荐搭配**：语音识别用默认本地 SenseVoice Small（零延迟、免费），AI 润色用 DeepSeek（效果好、费用极低）。

---

## 下载安装

### macOS

**[前往 Releases 下载](https://github.com/XL-shi/changyan/releases)**

| 芯片 | 文件 |
|------|------|
| Apple Silicon（M 系列） | `ChangYan_*_aarch64.dmg` |
| Intel | `ChangYan_*_x86_64.dmg` |

由于未使用 Apple 开发者签名，首次打开需要手动授权（只需一次）：

**方式一（推荐）** — 终端执行：
```bash
xattr -cr /Applications/ChangYan.app
```

**方式二** — 右键打开：拖入 Applications 后，右键 → 打开 → 弹窗中点"打开"

**方式三** — 系统设置：尝试打开后，进入「系统设置 → 隐私与安全性」，点击"仍要打开"

### Windows

下载并运行 `.msi` 安装包即可。

### Linux

```bash
# Debian / Ubuntu
sudo dpkg -i changyan_*.deb

# Fedora / RHEL
sudo rpm -i changyan-*.rpm

# AppImage
chmod +x ChangYan-*.AppImage && ./ChangYan-*.AppImage
```

---

## 快速上手

1. 安装后首次启动进入引导流程
2. 语音识别默认使用本地 SenseVoice Small，**首次会自动下载模型**（约 300MB），下载完成后离线可用
3. AI 润色步骤填入你的大模型 API Key（支持 DeepSeek、通义千问等），点测试通过后继续
4. 完成引导，设置全局快捷键（默认 `Fn` 键）
5. 在任意应用中按下快捷键，开口说话，松开后文字自动输入

---

## 隐私说明

- 语音数据直接发送给你配置的 STT 服务商（或完全在本地处理），**不经过畅言服务器**
- API Key 保存在本地，使用系统 Keychain 加密存储（macOS）
- 选择本地 SenseVoice + 本地 Ollama，可实现完全离线使用

---

## 本地开发

**环境要求**：Node.js 20+、Rust stable、[Tauri 前置依赖](https://v2.tauri.app/start/prerequisites/)

```bash
# 安装依赖
npm install

# 启动完整开发环境（前端 + Rust）
npm run tauri dev

# 测 onboarding / 模型下载隔离环境
npm run tauri:dev:devtest

# 仅调前端页面
npm run dev

# 生产构建
npm run tauri build
```

**发布版本时**，同步更新以下三处版本号：
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

---

## 常见问题

**语音识别需要联网吗？**
默认的 SenseVoice Small 完全在本地运行，无需联网。选择其他云端 STT 服务商则需要对应的 API Key 和网络。

**AI 润色是必须的吗？**
不是。关闭 AI 润色后，转写结果会直接输出，不经过大模型处理。

**支持哪些语言？**
SenseVoice Small 支持中、英、日、韩、粤语等多语言自动检测。云端服务商支持 99+ 语言。翻译模式支持 20+ 目标语言。

**API Key 安全吗？**
Key 只存储在本地设备，通过系统 Keychain 加密，请求直接发往服务商，不经过任何中转。

---

## License

[MIT](LICENSE)
