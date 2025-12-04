class NexusExplorer < Formula
  desc "A blazing-fast, GPU-accelerated file explorer built with Rust and GPUI"
  homepage "https://github.com/augani/nexus-explorer"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/augani/nexus-explorer/releases/download/v#{version}/NexusExplorer-aarch64-apple-darwin.dmg"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    end
    on_intel do
      url "https://github.com/augani/nexus-explorer/releases/download/v#{version}/NexusExplorer-x86_64-apple-darwin.dmg"
      sha256 "PLACEHOLDER_SHA256_X64"
    end
  end

  def install
    # Mount the DMG and copy the app
    prefix.install "Nexus Explorer.app"
    bin.write_exec_script "#{prefix}/Nexus Explorer.app/Contents/MacOS/Nexus Explorer"
  end

  def caveats
    <<~EOS
      Nexus Explorer has been installed to:
        #{prefix}/Nexus Explorer.app

      You can launch it from Applications or run:
        nexus-explorer
    EOS
  end

  test do
    assert_predicate prefix/"Nexus Explorer.app", :exist?
  end
end
