#!/bin/bash
set -e

APP_PATH="$1"

if [ -z "$APP_PATH" ]; then
  echo "Usage: $0 <path-to-app>"
  exit 1
fi

if [ ! -d "$APP_PATH" ]; then
  echo "Error: $APP_PATH does not exist"
  exit 1
fi

echo "🔐 Applying ad-hoc signature to $APP_PATH..."

# 递归签名所有二进制文件和动态库
find "$APP_PATH/Contents" -type f \( -perm -111 -o -name "*.dylib" -o -name "*.so" \) 2>/dev/null | while read binary; do
  echo "  Signing: $binary"
  codesign --force --sign - --timestamp=none "$binary" 2>/dev/null || true
done

# 签名整个 app bundle
codesign --force --deep --sign - --timestamp=none "$APP_PATH"

echo "✅ Ad-hoc signature applied"

# 验证签名
echo "🔍 Verifying signature..."
codesign -dv "$APP_PATH" 2>&1 | grep -E "(Signature=|Identifier=)" || true

echo ""
echo "⚠️  Note: This is an ad-hoc signature. Users will still need to:"
echo "   1. Run: xattr -cr '$APP_PATH'"
echo "   2. Or: Right-click → Open → Open Anyway"