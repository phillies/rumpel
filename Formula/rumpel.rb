# Homebrew formula; this repository doubles as its tap (see README, Installation).
# Bump url and sha256 after each release: curl -sL <url> | shasum -a 256
class Rumpel < Formula
  desc "Minimal GTK4/GStreamer video player"
  homepage "https://github.com/phillies/rumpel"
  url "https://github.com/phillies/rumpel/archive/refs/tags/v0.4.2.tar.gz"
  sha256 "d3da55a195e53f66c32cec53629a43f3a8733293ee5509baef6bbe679fc64d61"
  license "GPL-3.0-or-later"
  head "https://github.com/phillies/rumpel.git", branch: "main"

  depends_on "pkgconf" => :build
  depends_on "rust" => :build
  depends_on "glib"
  depends_on "gstreamer" # Homebrew bundles the good, bad, ugly and libav plugins here
  depends_on "gtk4"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_equal "rumpel #{version}", shell_output("#{bin}/rumpel --version").strip
  end
end
