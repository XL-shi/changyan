#!/bin/bash
# Build a .pkg installer for non-technical macOS users
# Usage: ./scripts/build-pkg.sh [path/to/ChangYan.app]
# If no app path is provided, looks in the default Tauri build output

set -e

VERSION=$(node -p "require('./package.json').version")
PKG_OUTPUT="ChangYan_${VERSION}.pkg"

# Resolve app path
if [ -n "$1" ]; then
  APP_PATH="$1"
else
  APP_PATH=$(find src-tauri/target -name "ChangYan.app" -maxdepth 5 2>/dev/null | head -n 1)
fi

if [ ! -d "$APP_PATH" ]; then
  echo "Error: ChangYan.app not found."
  echo "Run 'npm run tauri build' first, or pass the app path as an argument."
  exit 1
fi

echo "Building PKG installer v${VERSION}..."
echo "Source: $APP_PATH"

TMP_DIR=$(mktemp -d)
INSTALL_ROOT="${TMP_DIR}/root"
SCRIPTS_DIR="${TMP_DIR}/scripts"
mkdir -p "${INSTALL_ROOT}/Applications" "${SCRIPTS_DIR}"

# Apply ad-hoc signature to the app
codesign --force --deep --sign - "$APP_PATH"

# Copy app into package root
cp -R "$APP_PATH" "${INSTALL_ROOT}/Applications/"

# postinstall: remove quarantine so macOS doesn't block the app on first launch
cat > "${SCRIPTS_DIR}/postinstall" << 'EOF'
#!/bin/bash
xattr -cr /Applications/ChangYan.app
exit 0
EOF
chmod +x "${SCRIPTS_DIR}/postinstall"

# Build component package
pkgbuild \
  --root "${INSTALL_ROOT}" \
  --scripts "${SCRIPTS_DIR}" \
  --identifier "com.changyan.app" \
  --version "${VERSION}" \
  --install-location "/" \
  "${TMP_DIR}/component.pkg"

# Wrap in product archive (standard installer wizard UI)
productbuild \
  --package "${TMP_DIR}/component.pkg" \
  --identifier "com.changyan.app" \
  --version "${VERSION}" \
  "${PKG_OUTPUT}"

rm -rf "${TMP_DIR}"

echo ""
echo "Done: ${PKG_OUTPUT}"
echo "Users can double-click this file to install ChangYan."