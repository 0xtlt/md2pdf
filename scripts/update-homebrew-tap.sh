#!/usr/bin/env bash
# Update Formula/md2pdf.rb in 0xtlt/homebrew-tap for a published release.
#
# Required env:
#   GH_TOKEN   — PAT or fine-grained token with contents:write on 0xtlt/homebrew-tap
#   VERSION    — release version without leading "v" (e.g. 3.2.0)
#
# Optional env:
#   TAP_REPO    — defaults to 0xtlt/homebrew-tap
#   SOURCE_REPO — defaults to 0xtlt/md2pdf
set -euo pipefail

TAP_REPO="${TAP_REPO:-0xtlt/homebrew-tap}"
SOURCE_REPO="${SOURCE_REPO:-0xtlt/md2pdf}"
VERSION="${VERSION:?VERSION is required (e.g. 3.2.0)}"
VERSION="${VERSION#v}"

if [[ -z "${GH_TOKEN:-}" ]]; then
  echo "::warning::GH_TOKEN / HOMEBREW_TAP_TOKEN is not set; skipping Homebrew tap update"
  exit 0
fi

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

echo "Fetching SHA256SUMS for v${VERSION} from ${SOURCE_REPO}..."
gh release download "v${VERSION}" \
  --repo "${SOURCE_REPO}" \
  --pattern SHA256SUMS \
  --dir "${workdir}"

checksum() {
  local archive="$1"
  local sum
  sum="$(awk -v file="${archive}" '$2 == file { print $1; exit }' "${workdir}/SHA256SUMS")"
  if [[ -z "${sum}" ]]; then
    echo "Missing checksum for ${archive} in SHA256SUMS" >&2
    exit 1
  fi
  printf '%s' "${sum}"
}

MACOS_ARM64_SHA256="$(checksum md2pdf-macos-arm64.tar.gz)"
MACOS_X86_64_SHA256="$(checksum md2pdf-macos-x86_64.tar.gz)"
LINUX_X86_64_SHA256="$(checksum md2pdf-linux-x86_64.tar.gz)"

formula_path="${workdir}/md2pdf.rb"
cat >"${formula_path}" <<EOF
class Md2pdf < Formula
  desc "Fast, standalone Markdown to PDF converter written in Rust"
  homepage "https://github.com/${SOURCE_REPO}"
  version "${VERSION}"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/${SOURCE_REPO}/releases/download/v#{version}/md2pdf-macos-arm64.tar.gz"
      sha256 "${MACOS_ARM64_SHA256}"
    end
    on_intel do
      url "https://github.com/${SOURCE_REPO}/releases/download/v#{version}/md2pdf-macos-x86_64.tar.gz"
      sha256 "${MACOS_X86_64_SHA256}"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/${SOURCE_REPO}/releases/download/v#{version}/md2pdf-linux-x86_64.tar.gz"
      sha256 "${LINUX_X86_64_SHA256}"
    end
  end

  def install
    binary = Dir["*/md2pdf", "md2pdf"].find { |path| File.file?(path) }
    odie "expected md2pdf binary in release archive" unless binary
    bin.install binary => "md2pdf"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/md2pdf --version")
  end
end
EOF

tap_dir="${workdir}/homebrew-tap"
# Ensure git HTTPS pushes authenticate with GH_TOKEN (PAT for the tap repo).
gh auth setup-git
gh repo clone "${TAP_REPO}" "${tap_dir}" -- --depth 1
git -C "${tap_dir}" remote set-url origin "https://x-access-token:${GH_TOKEN}@github.com/${TAP_REPO}.git"

mkdir -p "${tap_dir}/Formula"
cp "${formula_path}" "${tap_dir}/Formula/md2pdf.rb"

readme="${tap_dir}/README.md"
if [[ -f "${readme}" ]]; then
  if ! grep -q 'brew install md2pdf' "${readme}"; then
    perl -pi -e 's/^(brew install vitrail)$/brew install md2pdf\n$1/m' "${readme}"
  fi
  if ! grep -q '| `md2pdf`' "${readme}"; then
    if ! grep -q '## Available Formulae' "${readme}"; then
      perl -0pi -e 's/## Available Casks\n/## Available Formulae\n\n| Formula | Description |\n|---------|-------------|\n| `md2pdf` | Fast, standalone Markdown to PDF converter |\n\n## Available Casks\n/' "${readme}"
    fi
  fi
fi

cd "${tap_dir}"
git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

git add Formula/md2pdf.rb
git add -u README.md 2>/dev/null || true

if git diff --cached --quiet; then
  echo "Homebrew tap already up to date for md2pdf ${VERSION}"
  exit 0
fi

git commit -m "md2pdf ${VERSION}"
git push origin HEAD

echo "Updated ${TAP_REPO} Formula/md2pdf.rb to ${VERSION}"
