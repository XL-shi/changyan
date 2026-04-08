# macOS 分发指南

## 分发方案对比

| 方案 | 目标用户 | 安装体验 | 成本 |
|------|---------|---------|------|
| **PKG 安装包** | 非技术用户 | 双击 → 向导安装，无需任何命令 | 免费 |
| **Homebrew Cask** | 开发者 / 技术用户 | 一行命令安装，自动更新 | 免费 |
| **DMG** | 备选 | 拖入 Applications，需右键 Open 一次 | 免费 |
| Developer ID | 所有用户 | 双击即用，无任何限制 | $99/年 |

**当前推荐**：PKG（非技术用户）+ Homebrew（技术用户），两者并行，不互相影响。

---

## 方案一：PKG 安装包（非技术用户）

用户只需双击 `.pkg` 文件，走标准 macOS 安装向导，全程无需 Terminal 命令。  
`postinstall` 脚本在安装时自动静默执行 `xattr -cr`，Gatekeeper 问题对用户不可见。

### 本地构建 PKG

```bash
# 先构建应用
npm run tauri build

# 生成 PKG
./scripts/build-pkg.sh
# 输出：ChangYan_{VERSION}.pkg
```

### 自动构建

推送 tag 后，`release.yml` 会自动构建 PKG 并上传到 GitHub Release。

---

## 方案二：Homebrew Cask（技术用户）

Cask 文件位于主仓库 `Casks/changyan.rb`，无需单独仓库。

### 用户安装命令

```bash
brew tap XL-shi/changyan https://github.com/XL-shi/changyan
brew install --cask changyan

# 后续更新
brew upgrade --cask changyan
```

### 工作原理

发布 Release 后，`update-homebrew.yml` 工作流自动：
1. 下载 Release 中的 `ChangYan_{VERSION}_aarch64.dmg`
2. 计算 SHA256
3. 更新 `Casks/changyan.rb`，提交到主分支

---

## 发布流程（完整步骤）

### 1. 对齐版本号

以下三个文件版本号必须一致：

```
package.json              → "version": "0.1.0"
src-tauri/Cargo.toml      → version = "0.1.0"
src-tauri/tauri.conf.json → "version": "0.1.0"
```

### 2. 提交并打 tag

```bash
git add .
git commit -m "chore: bump version to 0.1.0"
git push
git tag v0.1.0
git push origin v0.1.0
```

### 3. 等待 GitHub Actions 完成

`release.yml` 自动执行：
- 构建 macOS (arm64 + x64)、Windows、Linux
- 对 macOS 应用应用 ad-hoc 签名
- 构建 `ChangYan_{VERSION}.pkg`
- 创建 GitHub Release 草稿，上传所有产物

### 4. 发布 Release

去 [GitHub Releases](https://github.com/XL-shi/changyan/releases) 将草稿改为**正式发布**。

正式发布后触发 `update-homebrew.yml`，自动更新 Homebrew Cask。

### 5. 验证

```bash
# 验证 PKG 安装
open ChangYan_0.1.0.pkg

# 验证 Homebrew
brew tap XL-shi/changyan https://github.com/XL-shi/changyan
brew install --cask changyan
```

---

## 手动操作（工作流失败备用）

### 手动构建 PKG

```bash
npm run tauri build
./scripts/build-pkg.sh
```

### 手动更新 Homebrew Cask SHA256

```bash
curl -LO https://github.com/XL-shi/changyan/releases/download/v0.1.0/ChangYan_0.1.0_aarch64.dmg
shasum -a 256 ChangYan_0.1.0_aarch64.dmg
# 将输出的哈希值填入 Casks/changyan.rb 的 sha256 字段
git add Casks/changyan.rb
git commit -m "chore: update Homebrew Cask to 0.1.0"
git push
```

### 本地测试 Cask 语法

```bash
brew audit --cask Casks/changyan.rb
brew install --cask Casks/changyan.rb --force
brew uninstall --cask changyan --zap
```

---

## 升级路径（可选）

### Homebrew 短命令支持

如需 `brew tap XL-shi/changyan`（不带 URL），需创建独立的 `homebrew-changyan` 仓库，并配置 `TAP_GITHUB_TOKEN` secret 让工作流推送到该仓库。

### 提交到官方 Homebrew Cask

应用稳定后可提交至 [homebrew/homebrew-cask](https://github.com/Homebrew/homebrew-cask)，届时用户可直接 `brew install --cask changyan`：

1. Fork homebrew-cask 仓库
2. 将 `Casks/changyan.rb` 复制到 `Casks/c/changyan.rb`，填入真实 SHA256
3. 运行 `brew audit --cask --new changyan` 确认通过
4. 提交 PR，标题：`Add changyan 0.1.0`
5. 等待审核（通常 1-3 天）