---
name: render-markdown-pdf
description: Render Markdown documents into shareable PDFs using the md2pdf CLI. Use when the user asks to export, render, generate, or share Markdown as a PDF. Do not use for editing Markdown or generic PDF manipulation.
---

# Render Markdown as PDF

1. Identify the Markdown input file and the requested output path.
2. Check that `md2pdf` is available with `command -v md2pdf`.
3. Render the document with:

   ```bash
   md2pdf <input.md> --output <output.pdf>
   ```

4. Apply requested options such as title, author, page size, orientation, margins, accent color, or code theme.
5. If no output path is provided, use the source path with its extension changed to `.pdf`.
6. Use `--no-external` when remote image downloads are not wanted or network access is unavailable.
7. Verify that the PDF exists and report its absolute path.
8. Do not substitute Pandoc, LaTeX, a browser, or another converter unless the user explicitly asks.

If `md2pdf` is not installed, explain that it can be installed with one of the commands documented in this repository's README. Do not install it or choose a replacement converter without the user's approval.
