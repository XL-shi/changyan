# Distribution Options for ChangYan (macOS)

This directory contains documentation for macOS app distribution strategies.

## Quick Start

### Option 1: Homebrew Cask (Free, Best UX) ⭐ Recommended

**Cost**: $0
**User Experience**: One-line installation, automatic updates
**Setup Time**: 30 minutes

📖 **[Read Homebrew Guide](./HOMEBREW.md)**

**When to use**:
- Open source projects
- Technical user base
- Want great UX without Apple Developer costs
- Need automatic updates

**Quick setup**:
```bash
# Users install with:
brew tap XL-shi/changyan
brew install --cask changyan

# Users update with:
brew upgrade changyan
```

**Advantages over manual DMG**:
- ✅ No manual authorization needed
- ✅ Automatic updates built-in
- ✅ One-line install/uninstall
- ✅ Trusted by tech community

---

### Option 2: Ad-hoc Signing + Manual DMG (Free, Fallback)

**Cost**: $0
**User Experience**: Users run `xattr -cr /Applications/ChangYan.app` once
**Setup Time**: 5 minutes

📖 **[Read Ad-hoc Signing Guide](./ADHOC_SIGNING.md)**

**When to use**:
- Backup for Homebrew
- Users without Homebrew
- Quick local testing

**Quick setup**:
```bash
# Local build and sign
npm run tauri:build:signed

# Or use the standalone script
./scripts/build-and-sign.sh
```

GitHub Actions already configured - just push a tag:
```bash
git tag v0.1.15
git push origin v0.1.15
```

---

### Option 3: Developer ID Signing (Paid, Universal UX)

**Cost**: $99/year
**User Experience**: Double-click to install (no extra steps)
**Setup Time**: 1-2 hours (includes Apple enrollment)

📖 **[Read Developer ID Signing Guide](./CODESIGNING.md)**

**When to use**:
- Commercial products
- Large non-technical user base
- Need App Store distribution
- Want maximum trust/compatibility

**Setup**:
1. Enroll in [Apple Developer Program](https://developer.apple.com/programs/)
2. Create Developer ID certificate
3. Configure GitHub Secrets (see guide)
4. Push tag to release

---

## Comparison

| Feature | Homebrew | Ad-hoc + DMG | Developer ID |
|---------|----------|--------------|--------------|
| Cost | $0 | $0 | $99/year |
| Setup | 30 min | 5 min | 1-2 hours |
| User steps | `brew install` | `xattr` or right-click | None |
| Gatekeeper | ✅ Auto-handled | ❌ Blocked | ✅ Pass |
| Auto-update | ✅ `brew upgrade` | ❌ | ✅ Full |
| Target audience | Tech users | Tech users | Everyone |
| Trust level | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

## Recommended Strategy

**Phase 1 (Now)** - Dual approach:
1. ✅ **Homebrew** for tech-savvy users (primary)
2. ✅ **Ad-hoc DMG** for others (fallback)

**Phase 2 (If commercial)** - Add Developer ID:
1. Keep Homebrew (free distribution)
2. Add Developer ID for wider reach
3. Consider Mac App Store if appropriate

## Current Status

✅ **Homebrew Cask configured** (see `Casks/changyan.rb`)
✅ **Ad-hoc signing enabled** in GitHub Actions
⚠️ Developer ID signing is configured but requires secrets

To enable Developer ID:
1. Follow the [Developer ID guide](./CODESIGNING.md)
2. Add secrets to GitHub repository
3. Next release will be automatically notarized

## Testing Locally

```bash
# Build and sign with ad-hoc
npm run tauri:build:signed

# Or use the full script (includes DMG creation)
./scripts/build-and-sign.sh

# Verify signature
codesign -dv src-tauri/target/release/bundle/macos/ChangYan.app

# Check what users will see
spctl -a -t exec -vv src-tauri/target/release/bundle/macos/ChangYan.app
```

## User Instructions (Current)

Include this in your README or Release notes:

```markdown
### macOS Installation

**First launch only** - Run one of these commands:

```bash
# Method 1 (Recommended)
xattr -cr /Applications/ChangYan.app

# Method 2 (Alternative)
# Right-click ChangYan.app → Open → Open Anyway
```

After first launch, open normally from Applications.
```

---

## Questions?

- Ad-hoc signing issues → [ADHOC_SIGNING.md](./ADHOC_SIGNING.md#faq)
- Developer ID setup → [CODESIGNING.md](./CODESIGNING.md#故障排除)
- GitHub Actions failing → Check workflow logs for signing step