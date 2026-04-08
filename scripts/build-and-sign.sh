#!/bin/bash
set -e

echo "🔨 Building ChangYan..."
npm run tauri build

# 查找生成的 .app
APP_PATH=$(find src-tauri/target/*/release/bundle/macos -name "*.app" -maxdepth 1 2>/dev/null | head -n 1)

if [ -z "$APP_PATH" ]; then
  echo "❌ No .app bundle found"
  exit 1
fi

echo "📦 Found: $APP_PATH"

# Ad-hoc 签名
echo "🔐 Applying ad-hoc signature..."
find "$APP_PATH/Contents" -type f \( -perm -111 -o -name "*.dylib" -o -name "*.so" \) 2>/dev/null | while read binary; do
  codesign --force --sign - --timestamp=none "$binary" 2>/dev/null || true
done
codesign --force --deep --sign - --timestamp=none "$APP_PATH"

# 验证
echo "🔍 Verifying..."
codesign -dv "$APP_PATH" 2>&1 | grep "Signature=" || true

# 创建 DMG（可选）
if command -v hdiutil &> /dev/null; then
  echo "💿 Creating DMG..."
  DMG_NAME="ChangYan-$(date +%Y%m%d).dmg"
  hdiutil create -volname "ChangYan" -srcfolder "$APP_PATH" -ov -format UDZO "$DMG_NAME"
  echo "💿 DMG: $DMG_NAME"
fi

echo "✅ Done!"
echo "📁 App: $APP_PATH"
echo ""
echo "Test installation:"
echo "  1. Open the DMG (if created)"
echo "  2. Drag ChangYan.app to Applications"
echo "  3. Run: xattr -cr /Applications/ChangYan.app"
echo "  4. Open ChangYan from Applications"