#let inv = json(bytes(sys.inputs.invoice_json))

#set page(paper: "a4", margin: 2cm)
#set text(font: "Inter", size: 10pt)
#set par(justify: false, leading: 0.55em)

#let muted = rgb("#666666")
#let border = rgb("#e0e0e0")

// Header
#grid(
  columns: (1fr, 1fr),
  align: (left + top, right + top),
  [
    #if inv.logo_path != none [
      #image(inv.logo_path, height: 40pt)
    ] else [
      #text(size: 16pt, weight: "bold")[#inv.sender.name]
    ]
  ],
  [
    #text(size: 24pt, weight: "bold", fill: rgb("#333333"))[INVOICE] \
    #text(size: 14pt, fill: muted)[\# #inv.number]
  ],
)

#v(1cm)
#text(weight: "bold")[#inv.sender.name]
#if inv.sender.address_lines.len() > 0 [
  \ #for l in inv.sender.address_lines [#l \ ]
]

#v(0.8cm)

#grid(
  columns: (1fr, 1fr, 1fr),
  column-gutter: 20pt,
  [
    #text(weight: "bold")[Bill To] \
    #for l in inv.bill_to_lines [#l \ ]
  ],
  [
    #text(weight: "bold")[Ship To] \
    #for l in inv.ship_to_lines [#l \ ]
  ],
  grid(
    columns: (auto, 1fr),
    column-gutter: 10pt,
    row-gutter: 4pt,
    text(weight: "bold")[Date:], align(right)[#inv.date_display],
    text(weight: "bold")[PO Number:], align(right)[#inv.po_number],
    text(weight: "bold")[Balance Due:], align(right)[#text(weight: "bold")[#inv.balance_due_display]],
  ),
)

#v(1cm)

#table(
  columns: (1fr, auto, auto, auto),
  align: (left, right, right, right),
  stroke: (x, y) => if y == 0 { none } else { (bottom: 0.5pt + border) },
  fill: (col, row) => if row == 0 { rgb("#333333") } else { none },
  inset: 8pt,

  text(fill: white, weight: "bold")[Item],
  text(fill: white, weight: "bold")[Quantity],
  text(fill: white, weight: "bold")[Rate],
  text(fill: white, weight: "bold")[Amount],
  ..for item in inv.items {
    (
      item.description,
      item.quantity_display,
      item.rate_display,
      item.amount_display,
    )
  }
)

#v(1cm)

#align(right)[
  #grid(
    columns: (auto, 80pt),
    row-gutter: 8pt,
    align: (right, right),
    [Subtotal:], [#inv.subtotal_display],
    [#inv.tax_label:], [#inv.tax_display],
    text(weight: "bold")[Total:], text(weight: "bold")[#inv.total_display],
  )
]

#v(1.5cm)

#if inv.notes_lines.len() > 0 [
  #text(weight: "bold")[Notes] \
  #for l in inv.notes_lines [#l \ ]
]
#if inv.tax_note != none [
  #v(6pt)
  #text(size: 9pt, fill: muted)[#inv.tax_note]
]
