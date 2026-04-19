# invoicegen

<p align="center">
  <img src="meta/demo.png" alt="Rendered invoice PDF preview" width="720" />
</p>

`invoicegen` is a CLI that renders invoices from YAML to PDF. Write the invoice
as a small YAML file, and get a clean PDF rendered by [Typst](https://typst.app)
with embedded fonts, with no system dependencies and no LaTeX, no headless
browser.

Designed for solo contractors and small consultancies who want invoices under
version control and out of spreadsheet tools.

[Install](#install) · [Usage](#usage) · [Configuration](#configuration) ·
[Invoice YAML](#invoice-yaml) · [CLI reference](#cli-reference)

## Features

- **YAML-first invoices**: keep invoices as small, diffable text files instead
  of spreadsheets, with a structure that works well in version control
- **Reusable config and invoice-local overrides**: define sender details,
  defaults, and client templates once, then override them per invoice when
  needed for one-off billing cases
- **Self-contained PDF rendering**: single binary with embedded Typst template
  and fonts, with no LaTeX, browser, or system font setup required
- **Accurate money and tax handling**: `rust_decimal`-based totals plus
  locale-aware currency formatting for predictable invoice math
- **Practical file-based workflow**: works with regular files or stdin,
  preserves input-based output naming, and resolves output and asset paths
  predictably

## Install

### Quick install

```sh
curl -fsSL https://raw.githubusercontent.com/raine/invoicegen/main/scripts/install.sh | bash
```

### Homebrew (macOS/Linux)

```sh
brew install raine/invoicegen/invoicegen
```

### Cargo

```sh
cargo install invoicegen
```

The tool is a single binary with the Typst template and Inter fonts embedded, so
the installed binary is self-contained.

## Usage

### Quick start

This example is fully self-contained and does not require any global config.

### 1. Write an invoice YAML

```yaml
# examples/demo.yaml
number: 1
date: 2026-04-18
sender:
  name: 'Meridian Studio Ltd.'
  address: |
    18 Foundry Lane
    Helsinki 00140
    Finland
client:
  bill_to: |
    Brightleaf Systems
  ship_to: |
    Accounts Payable
    Brightleaf Systems
    88 Market Square
    Dublin D02
    Ireland
  default_rate: 100.00
po_number: 'BLS-APR-2026-17'
notes: 'Retainer for April platform support'
tax_rate: 0
tax_note: 'VAT 0% for demonstration purposes only'
items:
  - description: 'Product platform engineering support and delivery consulting'
    quantity: 160
```

### 2. Generate the PDF

```sh
invoicegen generate examples/demo.yaml
# → Wrote examples/demo.pdf
```

When `-o` is omitted, output goes to `<input-filename>.pdf` beside the invoice
file. If `defaults.output_dir` is set in config, that directory is used instead.

### Stdin mode

Pipe generated YAML into `invoicegen generate -`:

```sh
cat invoices/2026-04.yaml | invoicegen generate -
```

When reading from stdin, relative paths in the invoice YAML and the default
output directory resolve from the current working directory. In stdin mode, the
fallback output filename remains `invoice-<number>.pdf`.

### If you invoice regularly

Move shared sender details, defaults, and client templates into global config so
your monthly invoice YAML files can stay smaller.

### 1. Scaffold a config

```sh
invoicegen init
```

This writes a starter config to `~/.config/invoicegen/config.yaml` with a sample
sender, defaults, and an `example-client` template. Edit it to match your
business.

Once config is in place, your invoice YAML can reference a client template or
override parts of it inline.

### Using invoicegen with AI agents

The `invoicegen docs` command is a good piece of context to give an AI agent
before asking it to create invoices for you. It explains the YAML format,
config structure, and CLI usage, so the agent can generate invoice YAML and
tell you which `invoicegen generate ...` command to run.

A practical prompt is: "Read `invoicegen docs`, then make me an invoice for
CLIENT for WORK."

## Configuration

The global config lives at `$XDG_CONFIG_HOME/invoicegen/config.yaml` (default
`~/.config/invoicegen/config.yaml`).

### Example config

```yaml
sender:
  name: 'Your Company Ltd.'
  address: |
    123 Main Street
    City, Country
  logo: ~/.config/invoicegen/logo.svg # optional; SVG, PNG, or JPEG

defaults:
  currency: EUR
  date_format: '%b %-d, %Y'
  tax_rate: 0
  tax_note: 'VAT 0%, Export of goods or services'

clients:
  example-client:
    bill_to: |
      Example Client Inc.
      123 Example St
      City, State 12345
    ship_to: |
      Same as bill_to
    default_rate: 100.00
```

### Fields

#### `sender`

- `name` (string): company or individual name
- `address` (multi-line string, optional): printed under the sender name
- `logo` (path, optional): path to an SVG/PNG/JPEG; `~` is expanded. Rendered in
  the header of every invoice.

#### `defaults`

- `currency` (string): `EUR`, `USD`, or `GBP`, used to pick the symbol
- `date_format` (string): `jiff` strftime pattern (e.g. `%b %-d, %Y`)
- `output_dir` (path, optional): where PDFs land when `-o` is omitted. Relative
  paths resolve from the invoice file's directory. If omitted, the PDF is
  written beside the invoice file.
- `tax_rate` (decimal): percent (e.g. `24`). Defaults to `0`.
- `tax_note` (string, optional): small italic note printed below the totals
  block, handy for VAT disclaimers.

#### `clients`

A map of client keys to templates. Each template has:

- `bill_to` (multi-line string): printed in the "Bill To" block
- `ship_to` (multi-line string, optional): printed in the "Ship To" block
- `default_rate` (decimal, optional): used when a line item omits `rate`

## Invoice YAML

```yaml
number: 17 # required, integer
date: 2026-04-18 # required, YYYY-MM-DD
po_number: '001-015275' # optional
notes: | # optional, printed below the item table
  Thanks for the work this month.
tax_rate: 24 # optional, overrides defaults.tax_rate
tax_note: 'Reverse charge' # optional, overrides defaults.tax_note

sender: # optional; replaces the global sender block for this invoice
  name: 'Your Company Ltd.'
  address: |
    123 Main Street
    City, Country
  logo: ./logo.svg # resolved relative to the YAML file

client: # optional alternative to the string form above
  template: example-client # optional; start from a config client template
  bill_to: |
    One-off client
    Some address
  ship_to: |
    ...
  default_rate: 150

items: # required, at least one
  - description: 'Consulting'
    quantity: 10
    rate: 150.00 # optional; falls back to client default_rate
  - description: 'Extra review'
    quantity: 2
    rate: 200.00
```

`client` accepts either:

- a string like `client: example-client` to use a config client template as-is
- an object to override a template or define invoice-local client details
  directly

Invoice-local `sender` replaces the global sender block when present.
Invoice-local `client` data overrides global config or a referenced client
template. That keeps invoices portable: a YAML + `logo.svg` pair can travel
together, and the invoice still renders identically on a machine with no global
config.

## CLI reference

```
Generate PDF invoices from YAML

Usage: invoicegen <COMMAND>

Commands:
  init      Scaffold a starter config at ~/.config/invoicegen/config.yaml
  generate  Render an invoice YAML file to PDF
```

### `invoicegen generate`

```
Usage: invoicegen generate [OPTIONS] <FILE>

Arguments:
  <FILE>  Path to the invoice YAML file, or '-' to read YAML from stdin

Options:
  -o, --output <OUTPUT>  Output PDF path
                         (default: <input-filename>.pdf beside the invoice file)
```

## Development

```sh
just check     # fmt, clippy, build, test in parallel
just run -- generate examples/2026-04.yaml
```

## Related projects

- [workmux](https://github.com/raine/workmux): Git worktrees + tmux windows for
  parallel AI agent workflows
- [claude-history](https://github.com/raine/claude-history): Search and view
  Claude Code conversation history with fzf
- [git-surgeon](https://github.com/raine/git-surgeon): Non-interactive
  hunk-level git staging for AI agents
