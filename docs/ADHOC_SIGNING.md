# Ad-hoc Signing 配置指南（免费方案）

## 什么是 Ad-hoc Signing？

Ad-hoc signing 是一种**不需要 Apple Developer Program** 的本地签名方式：
- ✅ 免费，无需付费账号
- ✅ 可以在本地 Mac 上开发和测试
- ❌ **无法通过 Gatekeeper 验证**（用户仍需手动信任）
- ❌ 无法公证（notarization）
- ❌ 无法提交到 App Store

**适用场景**：开源项目、个人工具、内部分发，不追求"开箱即用"的用户体验。

## 方案对比

| 方案 | 成本 | 用户体验 | 适用场景 |
|---|---|---|---|
| **无签名**（当前） | $0 | 需要 `xattr -cr` 命令 | 技术用户 |
| **Ad-hoc Signing** | $0 | 需要 `xattr -cr` 或右键打开 | 技术用户 + 轻微改善 |
| **Developer ID** | $99/年 | 双击即可打开 | 商业产品、大规模分发 |

---

## 配置方法

### 方案 A：Tauri 配置自动签名（推荐）

修改 `src-tauri/tauri.conf.json`：

```json
{
  "bundle": {
    "macOS": {
      "minimumSystemVersion": "10.15",
      "signingIdentity": "-",
      "entitlements": "./Entitlements.plist",
      "infoPlist": "./Info.plist"
    }
  }
}
```

**关键**：`"signingIdentity": "-"` 表示使用 ad-hoc 签名。

### 方案 B：手动签名脚本（更灵活）

创建签名脚本 `scripts/adhoc-sign.sh`：

```bash
#!/bin/bash
set -e

APP_PATH="$1"

if [ -z "$APP_PATH" ]; then
  echo "Usage: $0 <path-to-app>"
  exit 1
fi

echo "🔐 Applying ad-hoc signature to $APP_PATH..."

# 递归签名所有二进制文件和动态库
find "$APP_PATH/Contents" -type f \( -perm -111 -o -name "*.dylib" -o -name "*.so" \) | while read binary; do
  echo "  Signing: $binary"
  codesign --force --sign - --timestamp=none "$binary" 2>/dev/null || true
done

# 签名整个 app bundle
codesign --force --deep --sign - --timestamp=none "$APP_PATH"

echo "✅ Ad-hoc signature applied"

# 验证签名
echo "🔍 Verifying signature..."
codesign -dv "$APP_PATH" 2>&1 | grep -E "(Signature=|Identifier=)"

echo ""
echo "⚠️  Note: This is an ad-hoc signature. Users will still need to:"
echo "   1. Run: xattr -cr '$APP_PATH'"
echo "   2. Or: Right-click → Open → Open Anyway"
```

赋予执行权限：
```bash
chmod +x scripts/adhoc-sign.sh
```

使用方法：
```bash
# 本地构建后签名
npm run tauri build
./scripts/adhoc-sign.sh "src-tauri/target/release/bundle/macos/ChangYan.app"
```

### 方案 C：GitHub Actions 自动签名

修改 `.github/workflows/release.yml`，在 `tauri-action` 之后添加签名步骤：

```yaml
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.RELEASE_TOKEN || secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.event.inputs.tag || github.ref_name }}
          releaseName: 'OpenTypeless ${{ github.event.inputs.tag || github.ref_name }}'
          releaseBody: 'See the assets below to download and install.'
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}

      # Ad-hoc signing for macOS (only runs on macOS runners)
      - name: Ad-hoc sign macOS app
        if: matrix.platform == 'macos-latest'
        run: |
          APP_PATH=$(find src-tauri/target/*/release/bundle/macos -name "*.app" -maxdepth 1 | head -n 1)
          if [ -n "$APP_PATH" ]; then
            echo "Applying ad-hoc signature to $APP_PATH"

            # Sign all binaries recursively
            find "$APP_PATH/Contents" -type f \( -perm -111 -o -name "*.dylib" -o -name "*.so" \) | while read binary; do
              codesign --force --sign - --timestamp=none "$binary" 2>/dev/null || true
            done

            # Sign the app bundle
            codesign --force --deep --sign - --timestamp=none "$APP_PATH"

            # Verify
            codesign -dv "$APP_PATH" 2>&1

            echo "✅ Ad-hoc signature applied"
          else
            echo "⚠️  No .app bundle found"
          fi
```

---

## 验证签名

### 本地验证

```bash
# 检查是否已签名
codesign -dv /path/to/ChangYan.app

# 输出示例（ad-hoc）：
# Executable=/path/to/ChangYan.app/Contents/MacOS/ChangYan
# Identifier=com.changyan.app
# Format=app bundle with Mach-O universal (arm64 x86_64)
# CodeDirectory v=20500 size=... flags=0x2(adhoc) hashes=...
# Signature=adhoc
# Info.plist=not bound

# 完整验证
codesign --verify --deep --strict --verbose=2 /path/to/ChangYan.app

# Gatekeeper 评估（会失败，因为是 ad-hoc）
spctl -a -t exec -vv /path/to/ChangYan.app
# 输出：rejected (the code is valid but does not seem to be an app)
```

### CI 构建验证

在 GitHub Actions 日志中查找：
```
✅ Ad-hoc signature applied
Signature=adhoc
```

---

## 用户安装指引（更新版）

更新 README 和 Release 说明：

```markdown
### macOS Installation

The app is signed with an ad-hoc signature (no Apple Developer account required).

**First launch requires one of these methods**:

**Method 1 (Recommended)** - Remove quarantine flag:
\`\`\`bash
xattr -cr /Applications/ChangYan.app
\`\`\`

**Method 2** - Right-click open:
1. Right-click on `ChangYan.app`
2. Select "Open"
3. Click "Open" in the dialog (appears only on first launch)

**Method 3** - System Settings:
1. Try to open the app (will show error)
2. Open `System Settings → Privacy & Security`
3. Click "Open Anyway" under Security section

After the first launch, you can open the app normally.
```

---

## 自动化构建脚本

创建 `scripts/build-and-sign.sh`（本地开发使用）：

```bash
#!/bin/bash
set -e

echo "🔨 Building ChangYan..."
npm run tauri build

# 查找生成的 .app
APP_PATH=$(find src-tauri/target/*/release/bundle/macos -name "*.app" -maxdepth 1 | head -n 1)

if [ -z "$APP_PATH" ]; then
  echo "❌ No .app bundle found"
  exit 1
fi

echo "📦 Found: $APP_PATH"

# Ad-hoc 签名
echo "🔐 Applying ad-hoc signature..."
find "$APP_PATH/Contents" -type f \( -perm -111 -o -name "*.dylib" -o -name "*.so" \) | while read binary; do
  codesign --force --sign - --timestamp=none "$binary" 2>/dev/null || true
done
codesign --force --deep --sign - --timestamp=none "$APP_PATH"

# 验证
echo "🔍 Verifying..."
codesign -dv "$APP_PATH" 2>&1 | grep "Signature="

# 创建 DMG（可选）
echo "💿 Creating DMG..."
DMG_NAME="ChangYan-$(date +%Y%m%d).dmg"
hdiutil create -volname "ChangYan" -srcfolder "$APP_PATH" -ov -format UDZO "$DMG_NAME"

echo "✅ Done!"
echo "📁 App: $APP_PATH"
echo "💿 DMG: $DMG_NAME"
echo ""
echo "Test installation:"
echo "  1. Open the DMG"
echo "  2. Drag ChangYan.app to Applications"
echo "  3. Run: xattr -cr /Applications/ChangYan.app"
echo "  4. Open ChangYan from Applications"
```

---

## FAQ

### Q: Ad-hoc 签名和无签名有什么区别？

A: 从 Gatekeeper 角度看，**效果相同**（都需要用户手动信任）。但 ad-hoc 签名的好处：
- 验证应用未被篡改（签名后修改会导致签名失效）
- 部分安全工具会检测签名状态
- 为将来升级到 Developer ID 打好基础（签名流程已就位）

### Q: 为什么不直接禁用 Gatekeeper？

A: 不建议告诉用户运行 `sudo spctl --master-disable`，这会完全关闭 macOS 的安全保护。`xattr -cr` 只针对单个应用解除限制，更安全。

### Q: 能否让 CI 自动生成签名？

A: 可以（见方案 C），但生成的仍是 ad-hoc 签名，用户体验不变。主要好处是确保发布的二进制已签名。

### Q: 何时应该升级到 Developer ID？

当你的项目满足以下任一条件：
- 用户群体包含非技术人员
- 需要"开箱即用"的体验
- 计划商业化或大规模推广
- 需���自动更新功能（需要签名）

---

## 参考资料

- [Apple Code Signing Guide](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/)
- [Tauri macOS Bundle](https://v2.tauri.app/reference/config/#macosconfig)
- [codesign man page](https://www.manpagez.com/man/1/codesign/)