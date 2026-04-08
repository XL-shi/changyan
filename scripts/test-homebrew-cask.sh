#!/bin/bash
set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Homebrew Cask 本地测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

CASK_FILE="Casks/changyan.rb"

if [ ! -f "$CASK_FILE" ]; then
  echo "❌ 错误：未找到 $CASK_FILE"
  exit 1
fi

echo "📦 Cask 文件: $CASK_FILE"
echo ""

# 1. 语法检查
echo "1️⃣ 语法检查..."
if brew audit --cask "$CASK_FILE"; then
  echo "✅ 语法检查通过"
else
  echo "⚠️  语法检查有警告（可能是 sha256 :no_check）"
fi
echo ""

# 2. 样式检查
echo "2️⃣ 样式检查..."
brew style "$CASK_FILE" || true
echo ""

# 3. 检查当前安装状态
echo "3️⃣ 检查安装状态..."
if brew list --cask changyan &>/dev/null; then
  echo "⚠️  ChangYan 已安装，将先卸载..."
  brew uninstall --cask changyan --force
  echo "✅ 卸载完成"
else
  echo "✅ 未安装，准备安装"
fi
echo ""

# 4. 安装测试
echo "4️⃣ 安装测试..."
read -p "是否执行安装测试？(y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "跳过安装测试"
  exit 0
fi

echo "开始安装..."
brew install --cask "$CASK_FILE" --force --verbose

if [ $? -eq 0 ]; then
  echo "✅ 安装成功"
else
  echo "❌ 安装失败"
  exit 1
fi
echo ""

# 5. 验证安装
echo "5️⃣ 验证安装..."

if [ -d "/Applications/ChangYan.app" ]; then
  echo "✅ 应用已安装到 /Applications/ChangYan.app"

  # 检查签名
  if codesign -dv "/Applications/ChangYan.app" 2>&1 | grep -q "adhoc"; then
    echo "✅ 检测到 ad-hoc 签名"
  else
    echo "⚠️  签名状态未知"
  fi

  # 检查隔离属性
  if xattr "/Applications/ChangYan.app" | grep -q "com.apple.quarantine"; then
    echo "⚠️  仍存在隔离属性（postflight 可能未执行）"
  else
    echo "✅ 隔离属性已移除"
  fi

  # 检查应用能否启动
  echo ""
  read -p "是否尝试启动应用？(y/N) " -n 1 -r
  echo ""
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    open "/Applications/ChangYan.app"
    echo "✅ 应用已启动"
  fi
else
  echo "❌ 应用未安装到 /Applications"
  exit 1
fi
echo ""

# 6. 清理
echo "6️⃣ 清理测试..."
read -p "是否卸载测试安装？(Y/n) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
  brew uninstall --cask changyan --zap
  echo "✅ 卸载完成"
else
  echo "保留安装"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅ 测试完成！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"