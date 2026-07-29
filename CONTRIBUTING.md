# Contributing to md2pdf

Thank you for contributing. Changes should remain focused, tested, and
compatible with the standalone nature of the executable.

## Set up the project

Rust stable and Git are required:

```console
git clone https://github.com/0xtlt/md2pdf.git
cd md2pdf
cargo test --locked
```

## Workflow

1. Create a short branch from `main`.
2. Add or update tests before changing behavior.
3. Format the code with `cargo fmt`.
4. Run the complete quality gate.
5. Open a pull request describing the problem and the solution.

## Quality gate

```console
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps
```

For visual changes, also generate `example.pdf`, render every page to images
with Poppler, and check for overflow, collisions, clipping, and inconsistent
spacing.

## Style

- prefer short functions and explicit names;
- document every public API;
- avoid dependencies when a small, safe implementation is sufficient;
- keep all user-facing CLI messages in English;
- do not add an external runtime to the execution path.

## Tests

A bug fix should include a test that fails before the fix. Integration tests
should invoke the real executable and use a temporary directory.

## Releases

Tagged versions (`v*`) run `.github/workflows/release.yml`, which builds
platform archives, publishes a GitHub release, and updates
[`0xtlt/homebrew-tap`](https://github.com/0xtlt/homebrew-tap).

The tap update job needs a repository secret named `HOMEBREW_TAP_TOKEN`: a
classic PAT or fine-grained token with **Contents: Read and write** on
`0xtlt/homebrew-tap`. Without that secret the release still publishes; the tap
step is skipped with a warning.

Install from the tap with the fully-qualified name
`brew install 0xtlt/tap/md2pdf`. Plain `brew install md2pdf` resolves to
Homebrew core's unrelated Go package.
