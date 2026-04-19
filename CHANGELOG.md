# Changelog

## v0.1.2 (2026-04-19)

- Get clearer error messages when invoice or config files are invalid, including
  the exact field that failed and actionable help for common mistakes.

## v0.1.1 (2026-04-18)

- Read the built-in documentation with `invoicegen docs`, so install and usage
  help is available directly in the CLI.
- Get PDF files named after the input invoice by default, with output written
  next to the invoice unless you set `defaults.output_dir`.
- Use `~` in invoice and config paths more reliably, including sender logos and
  output locations.
- Override sender details per invoice without losing invoice-specific values.

## v0.1.0 (2026-04-18)

- Initial release of `invoicegen`, a CLI that renders invoices from YAML to PDF
  using Typst with embedded fonts, decimal-accurate money math, and no LaTeX or
  headless browser dependency
