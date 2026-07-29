class Md2pdf < Formula
  desc "Fast, standalone Markdown to PDF converter written in Rust"
  homepage "https://github.com/0xtlt/md2pdf"
  version "3.2.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/0xtlt/md2pdf/releases/download/v#{version}/md2pdf-macos-arm64.tar.gz"
      sha256 "44730809a79a860fc3df2003f3abfac2a2d793eb98a3f82c96e60c4bb2a9c79a"
    end
    on_intel do
      url "https://github.com/0xtlt/md2pdf/releases/download/v#{version}/md2pdf-macos-x86_64.tar.gz"
      sha256 "2a2f93280c7d613788a823c8653717e31b3f134cef7c340893005af208d17f6b"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/0xtlt/md2pdf/releases/download/v#{version}/md2pdf-linux-x86_64.tar.gz"
      sha256 "557b670a148de266bbc8dba3b8d7ab2f886bd6cfba275533cb6a7e5ff8918368"
    end
  end

  def install
    binary = Dir["*/md2pdf", "md2pdf"].find { |path| File.file?(path) }
    odie "expected md2pdf binary in release archive" unless binary
    bin.install binary => "md2pdf"
  end

  def caveats
    <<~EOS
      This is the Rust md2pdf from 0xtlt/md2pdf.

      Homebrew core ships a different Go-based md2pdf (solworktech/md2pdf).
      Prefer the tap formula once published:

        brew install 0xtlt/tap/md2pdf

      If `md2pdf --help` mentions md2pdf.go or only -i/-o flags, uninstall the
      core package first, then reinstall this formula.
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/md2pdf --version")
  end
end
