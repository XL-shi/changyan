## 🎉 ChangYan v{VERSION}

### 📥 下载安装

#### macOS

**重要提示**：由于应用使用免费的 ad-hoc 签名，首次打开时需要手动授权。请选择以下任一方法：

**方法一（最简单）** - 使用命令行：
```bash
xattr -cr /Applications/ChangYan.app
```

**方法二** - 右键打开：
1. 将 `ChangYan.app` 拖到 Applications 文件夹
2. 右键点击 → 选择"打开"
3. 在弹出的对话框中点击"打开"按钮
4. 仅首次需要，之后可正常打开

**方法三** - 系统设置：
1. 尝试打开应用（会显示错误）
2. 打开 `系统设置 → 隐私与安全性`
3. 在"安全性"部分点击"仍要打开"

#### Windows

直接下载 `.msi` 或 `.exe` 安装包，双击安装即可。

#### Linux

下载 `.deb`、`.rpm` 或 `.AppImage` 文件：
- Debian/Ubuntu: `sudo dpkg -i changyan_*.deb`
- Fedora/RHEL: `sudo rpm -i changyan-*.rpm`
- AppImage: `chmod +x changyan-*.AppImage && ./changyan-*.AppImage`

### ✨ 新功能

- [待填写]

### 🐛 问题修复

- [待填写]

### 📝 完整更新日志

[查看完整更新日志](https://github.com/XL-shi/changyan/compare/v{PREV_VERSION}...v{VERSION})

---

### 🔒 关于代码签名

本项目使用 **ad-hoc 签名**（免费方案），无需付费的 Apple Developer 账号。这意味着：

- ✅ 应用是安全���，源代码完全开源
- ✅ 签名确保应用未被篡改
- ⚠️ macOS Gatekeeper 需要手动授权首次打开
- ℹ️ 若需要"开箱即用"的体验，需要 Apple Developer ID（$99/年）

如有问题，请查看 [安装指南](https://github.com/XL-shi/changyan#installation) 或提交 [Issue](https://github.com/XL-shi/changyan/issues)。