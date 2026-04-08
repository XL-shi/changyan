#!/bin/bash

# 首次 Homebrew 发布前的验证清单

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Homebrew 发布前验证清单"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━��━━━━━━━"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass_count=0
fail_count=0
warn_count=0

check_pass() {
  echo -e "${GREEN}✅ $1${NC}"
  ((pass_count++))
}

check_fail() {
  echo -e "${RED}❌ $1${NC}"
  ((fail_count++))
}

check_warn() {
  echo -e "${YELLOW}⚠️  $1${NC}"
  ((warn_count++))
}

# 1. 检查 Cask 文件存在
echo "1. 检查 Cask 文件..."
if [ -f "Casks/changyan.rb" ]; then
  check_pass "Cask 文件存在"
else
  check_fail "Cask 文件不存在"
fi
echo ""

# 2. 检查 tauri.conf.json 中的签名配置
echo "2. 检查 Tauri 签名配置..."
if grep -q '"signingIdentity": "-"' src-tauri/tauri.conf.json; then
  check_pass "Ad-hoc 签名已配置"
else
  check_warn "Ad-hoc 签名未配置（将使用默认设置）"
fi
echo ""

# 3. 检查 GitHub Release 工作流
echo "3. 检查 GitHub Actions 工作流..."
if [ -f ".github/workflows/release.yml" ]; then
  check_pass "Release 工作流存在"

  if grep -q "Apply ad-hoc signature" .github/workflows/release.yml; then
    check_pass "Ad-hoc 签名步骤已配置"
  else
    check_warn "Ad-hoc 签名步骤缺失"
  fi
else
  check_fail "Release 工作流不存在"
fi
echo ""

# 4. 检查 Homebrew 自动更新工作流
echo "4. 检查 Homebrew 自动更新工作流..."
if [ -f ".github/workflows/update-homebrew.yml" ]; then
  check_pass "Homebrew 自动更新工作流存在"
else
  check_warn "Homebrew 自动更新工作流不存在（需要手动更新）"
fi
echo ""

# 5. 检查版本号一致性
echo "5. 检查版本号一致性..."
PKG_VERSION=$(grep '"version"' package.json | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')
TAURI_VERSION=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')
CASK_VERSION=$(grep 'version "' Casks/changyan.rb | sed 's/.*version "\(.*\)".*/\1/')

echo "  package.json:        $PKG_VERSION"
echo "  tauri.conf.json:     $TAURI_VERSION"
echo "  Casks/changyan.rb:   $CASK_VERSION"

if [ "$PKG_VERSION" = "$TAURI_VERSION" ] && [ "$PKG_VERSION" = "$CASK_VERSION" ]; then
  check_pass "版本号一致"
else
  check_fail "版本号不一致"
fi
echo ""

# 6. 检查 Homebrew Tap 仓库
echo "6. 检查 Homebrew Tap 仓库..."
GITHUB_USER=$(git config --get remote.origin.url | sed 's/.*github.com[:/]\(.*\)\/.*/\1/')
TAP_REPO="homebrew-changyan"

echo "  期望的 Tap 仓库: https://github.com/$GITHUB_USER/$TAP_REPO"

if curl -sf "https://api.github.com/repos/$GITHUB_USER/$TAP_REPO" > /dev/null 2>&1; then
  check_pass "Homebrew Tap 仓库存在"
else
  check_warn "Homebrew Tap 仓库不存在（需要创建）"
  echo "    运行: ./scripts/setup-homebrew-tap.sh"
fi
echo ""

# 7. 检查 README 是否包含 Homebrew 安装说明
echo "7. 检查 README..."
if grep -q "brew install" README.md; then
  check_pass "README 包含 Homebrew 安装说明"
else
  check_warn "README 缺少 Homebrew 安装说明"
fi
echo ""

# 8. 检查是否有最新的 Git 标签
echo "8. 检查 Git 标签..."
LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "none")
echo "  最新标签: $LATEST_TAG"

if [ "$LATEST_TAG" != "none" ]; then
  check_pass "存在 Git 标签"
else
  check_warn "没有 Git 标签（首次发布需要创建）"
fi
echo ""

# 9. 检查 Release 文件命名格式（如果有标签）
if [ "$LATEST_TAG" != "none" ]; then
  echo "9. 验证 Release 文件命名..."
  TAG_VERSION="${LATEST_TAG#v}"

  echo "  检查 GitHub Release: $LATEST_TAG"
  echo "  期望的 DMG 文件名: ChangYan_${TAG_VERSION}_aarch64.dmg"
  echo ""
  echo "  ⚠️  请手动验证："
  echo "     访问: https://github.com/$GITHUB_USER/changyan/releases/tag/$LATEST_TAG"
  echo "     确认 DMG 文件名与 Cask 配置中的 URL 匹配"
  check_warn "需要手动验证 Release 文件名"
else
  echo "9. 跳过（无标签）"
  check_warn "首次发布后需要验证文件名格式"
fi
echo ""

# 总结
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  检查结果"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}通过: $pass_count${NC}"
echo -e "${YELLOW}警告: $warn_count${NC}"
echo -e "${RED}失败: $fail_count${NC}"
echo ""

if [ $fail_count -eq 0 ]; then
  echo "✅ 准备就绪！可以发布第一个版本。"
  echo ""
  echo "下一步："
  echo "  1. 创建 Homebrew Tap 仓库（如果未创建）"
  echo "     ./scripts/setup-homebrew-tap.sh"
  echo ""
  echo "  2. 提交并推送代码"
  echo "     git add ."
  echo "     git commit -m 'feat: add Homebrew Cask support'"
  echo "     git push"
  echo ""
  echo "  3. 创建并推送标签"
  echo "     git tag v$PKG_VERSION"
  echo "     git push origin v$PKG_VERSION"
  echo ""
  echo "  4. 等待 GitHub Actions 完成"
  echo "     访问: https://github.com/$GITHUB_USER/changyan/actions"
  echo ""
  echo "  5. 验证并发布 Release（从草稿改为正式）"
  echo "     访问: https://github.com/$GITHUB_USER/changyan/releases"
  echo ""
  echo "  6. Homebrew Cask 会自动更新（或手动更新）"
  echo ""
else
  echo "❌ 存在问题需要修复。请检查上述失败项。"
fi
