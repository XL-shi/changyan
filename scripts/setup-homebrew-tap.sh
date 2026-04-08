#!/bin/bash
set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Homebrew Tap 初始化向导"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "此脚本将帮助你创建一个 Homebrew Tap 仓库，"
echo "让用户可以通过 Homebrew 安装 ChangYan。"
echo ""

# 检查是否有未提交的更改
if ! git diff-index --quiet HEAD --; then
  echo "⚠️  警告：当前有未提交的更改"
  read -p "是否继续？(y/N) " -n 1 -r
  echo ""
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
  fi
fi

# 获取 GitHub 用户名
GITHUB_USER=$(git config --get user.name || echo "")
read -p "GitHub 用户名 [$GITHUB_USER]: " INPUT_USER
GITHUB_USER="${INPUT_USER:-$GITHUB_USER}"

if [ -z "$GITHUB_USER" ]; then
  echo "❌ 错误：需要 GitHub 用户名"
  exit 1
fi

TAP_REPO="homebrew-changyan"
TAP_URL="https://github.com/$GITHUB_USER/$TAP_REPO"

echo ""
echo "将创建以下仓库："
echo "  📦 名称: $TAP_REPO"
echo "  🔗 URL: $TAP_URL"
echo ""
read -p "是否继续？(Y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Nn]$ ]]; then
  exit 0
fi

# 步骤 1: 在 GitHub 创建仓库
echo ""
echo "1️⃣ 在 GitHub 创建 Tap 仓库..."
echo ""
echo "请手动执行以下步骤："
echo "  1. 访问 https://github.com/new"
echo "  2. 仓库名称: $TAP_REPO"
echo "  3. 描述: Homebrew tap for ChangYan"
echo "  4. 公开仓库"
echo "  5. 不添加 README、.gitignore 或 LICENSE"
echo "  6. 点击 'Create repository'"
echo ""
read -p "完成后按回车继续..."

# 步骤 2: 克隆并初始化仓库
echo ""
echo "2️⃣ 初始化本地仓库..."

TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

git clone "https://github.com/$GITHUB_USER/$TAP_REPO.git" || {
  echo "❌ 克隆失败，请检查仓库是否已创建"
  exit 1
}

cd "$TAP_REPO"

# 创建目录结构
mkdir -p Casks

# 复制 Cask 文件
cp "$(git rev-parse --show-toplevel 2>/dev/null || echo "$OLDPWD")/Casks/changyan.rb" Casks/

# 创建 README
cat > README.md << EOF
# ChangYan Homebrew Tap

Official Homebrew tap for [ChangYan](https://github.com/$GITHUB_USER/changyan) - AI-powered speech-to-text desktop app.

## Installation

\`\`\`bash
brew tap $GITHUB_USER/changyan
brew install --cask changyan
\`\`\`

## Upgrade

\`\`\`bash
brew upgrade --cask changyan
\`\`\`

## Uninstall

\`\`\`bash
brew uninstall --cask changyan
\`\`\`

## About

This tap provides easy installation of ChangYan on macOS without manual authorization steps.

For more information, visit the [main repository](https://github.com/$GITHUB_USER/changyan).
EOF

# 提交并推送
git add .
git commit -m "Initial commit: Add ChangYan cask"
git push -u origin main || git push -u origin master

echo ""
echo "✅ Tap 仓库已创建！"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  下一步操作"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1️⃣ 测试安装："
echo "   brew tap $GITHUB_USER/changyan"
echo "   brew install --cask changyan"
echo ""
echo "2️⃣ 更新 README.md，添加 Homebrew 安装说明"
echo ""
echo "3️⃣ 发布新版本时，自动更新 Cask："
echo "   - GitHub Actions 会自动更新版本和 SHA256"
echo "   - 也可以手动运行: ./scripts/update-homebrew-cask.sh"
echo ""
echo "4️⃣ （可选）提交到官方 Homebrew Cask："
echo "   - 参考 docs/HOMEBREW.md 的说明"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Tap 仓库位置: $TEMP_DIR/$TAP_REPO"
echo ""
read -p "按回车关闭..."