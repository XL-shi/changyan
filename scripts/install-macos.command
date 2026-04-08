#!/bin/bash

# macOS 一键安装脚本
# 双击此文件即可运行

cd "$(dirname "$0")"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ChangYan 安装脚本"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 查找 .app 文件
APP_NAME="ChangYan.app"
APP_SOURCE=""

# 检查是否在 DMG 中
if [ -f "../$APP_NAME" ]; then
  APP_SOURCE="../$APP_NAME"
elif [ -f "./$APP_NAME" ]; then
  APP_SOURCE="./$APP_NAME"
else
  echo "❌ 错误：未找到 $APP_NAME"
  echo ""
  echo "请确保此脚本与 ChangYan.app 在同一目录"
  echo "按任意键退出..."
  read -n 1
  exit 1
fi

APP_DEST="/Applications/$APP_NAME"

echo "📦 准备安装..."
echo "   源路径: $APP_SOURCE"
echo "   目标路径: $APP_DEST"
echo ""

# 如果已存在，询问是否覆盖
if [ -d "$APP_DEST" ]; then
  echo "⚠️  检测到已安装的版本"
  read -p "是否覆盖？(y/N) " -n 1 -r
  echo ""
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "取消安装"
    echo "按任意键退出..."
    read -n 1
    exit 0
  fi
  echo "🗑️  删除旧版本..."
  rm -rf "$APP_DEST"
fi

# 复制应用
echo "📋 复制应用到 Applications..."
cp -R "$APP_SOURCE" "$APP_DEST"

if [ ! -d "$APP_DEST" ]; then
  echo "❌ 复制失败"
  echo "按任意键退出..."
  read -n 1
  exit 1
fi

# 移除隔离属性
echo "🔓 移除隔离属性..."
xattr -cr "$APP_DEST" 2>/dev/null || true

# 验证
echo "🔍 验证安装..."
if codesign -dv "$APP_DEST" 2>/dev/null | grep -q "adhoc"; then
  echo "✅ 签名验证通过"
else
  echo "⚠️  未检测到签名（仍可使用）"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅ 安装完成！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 提示："
echo "   1. 在启动台或 Applications 文件夹中找到 ChangYan"
echo "   2. 首次打开可能需要右键 → 打开"
echo "   3. 或在系统设置 → 隐私与安全 → 点击「仍要打开」"
echo ""
echo "按任意键退出..."
read -n 1