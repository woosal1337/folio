# Homebrew cask for Folio — local-first meeting transcription for macOS.
#
# This repository doubles as its own Homebrew tap. Install with:
#
#   brew tap woosal1337/folio https://github.com/woosal1337/folio
#   brew install --cask folio
#
# The `version` and `sha256` lines below are updated automatically by the
# `update-homebrew-cask` job in .github/workflows/release.yml on every tagged
# release, so `brew upgrade --cask folio` tracks new releases.
cask "folio" do
  version "1.1.0"
  sha256 "987b7697df2fe10b637884feadeb82a7338fd3ccf3c5dd39940372e8bc188e32"

  url "https://github.com/woosal1337/folio/releases/download/v#{version}/Folio_#{version}_aarch64.dmg",
      verified: "github.com/woosal1337/folio/"
  name "Folio"
  desc "Local-first meeting transcription"
  homepage "https://github.com/woosal1337/folio"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on macos: :ventura

  app "Folio.app"

  zap trash: [
    "~/Library/Application Support/Folio",
    "~/Library/Caches/dev.folio.app",
    "~/Library/HTTPStorages/dev.folio.app",
    "~/Library/Preferences/dev.folio.app.plist",
    "~/Library/Saved Application State/dev.folio.app.savedState",
  ]
end
