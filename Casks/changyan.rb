cask "changyan" do
  version "0.1.0"
  sha256 :no_check # Will be updated on release

  url "https://github.com/XL-shi/changyan/releases/download/v#{version}/ChangYan_#{version}_aarch64.dmg",
      verified: "github.com/XL-shi/changyan/"
  name "ChangYan"
  desc "AI-powered speech-to-text and text polishing desktop app"
  homepage "https://github.com/XL-shi/changyan"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "ChangYan.app"

  postflight do
    # Remove quarantine attribute automatically
    system_command "/usr/bin/xattr",
                   args: ["-cr", "#{appdir}/ChangYan.app"],
                   sudo: false
  end

  uninstall quit: "com.changyan.app"

  zap trash: [
    "~/Library/Application Support/com.changyan.app",
    "~/Library/Caches/com.changyan.app",
    "~/Library/Preferences/com.changyan.app.plist",
    "~/Library/Saved Application State/com.changyan.app.savedState",
    "~/Library/WebKit/com.changyan.app",
  ]

  caveats <<~EOS
    ChangYan has been installed successfully!

    The app is signed with an ad-hoc signature (no Apple Developer account required).
    Homebrew has automatically removed the quarantine attribute, so you can launch it normally.

    To start ChangYan:
      1. Open from Applications folder or Spotlight
      2. Set up your STT and LLM API keys in Settings
      3. Configure your global hotkey
      4. Start dictating!

    For help and documentation, visit:
      https://github.com/XL-shi/changyan
  EOS
end