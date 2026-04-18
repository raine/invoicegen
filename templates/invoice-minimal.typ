#let inv = json.decode(sys.inputs.invoice_json)

#let paper = rgb("#fcfcfc")
#let ink = rgb("#383634")
#let ink-soft = rgb("#6b6762")
#let ink-faint = rgb("#7d7974")
#let hairline = rgb("#d6d3cf")

#let rule = line(length: 100%, stroke: 0.6pt + hairline)
#let vrule = box(width: 0.6pt, height: 22pt, fill: hairline)

#let mono-label(body) = text(
  font: "JetBrains Mono",
  size: 8.25pt,
  weight: 500,
  tracking: 0.12em,
  fill: ink-faint,
)[#body]

#set page(
  paper: "us-letter",
  margin: (top: 36pt, right: 42pt, bottom: 54pt, left: 42pt),
  fill: paper,
  footer: [
    #grid(
      columns: (1fr, auto),
      align: (left + bottom, right + bottom),
      [#mono-label([#inv.sender.name])],
      [#text(font: "Instrument Serif", style: "italic", size: 10.5pt, fill: ink-soft)[Thank you.]],
    )
  ],
)
#set text(font: "Inter", size: 10.5pt, fill: ink)
#set par(justify: false, leading: 0.55em)

#let muted-lines(lines) = {
  if lines.len() > 1 {
    for line in lines.slice(1) {
      text(size: 9.75pt, fill: ink-soft)[#line]
      linebreak()
    }
  }
}

#let party-block(label, lines) = [
  #mono-label(label)
  #v(9pt)
  #if lines.len() > 0 [
    #text(size: 9.75pt, weight: 500)[#lines.at(0)]
    #if lines.len() > 1 [
      #v(2pt)
      #muted-lines(lines)
    ]
  ]
]

#block(width: 100%)[
  #grid(
    columns: (1fr, 1fr),
    align: (left + top, right + top),
    column-gutter: 24pt,
    [
      #if inv.logo_path != none [
        #image(inv.logo_path, height: 24pt)
      ] else [
        #text(font: "Instrument Serif", size: 21pt)[#inv.sender.name]
      ]
    ],
    [
      #align(right)[
        #text(font: "Inter", size: 30pt, weight: 500, tracking: -0.035em)[Invoice]
        #v(6pt)
        #text(font: "JetBrains Mono", size: 9pt, tracking: 0.04em, fill: ink-soft)[No. #inv.number]
      ]
    ],
  )
]

#v(24pt)
#rule
#v(24pt)

#block(width: 100%)[
  #grid(
    columns: (1fr, 1fr, 1fr),
    column-gutter: 30pt,
    align: (left + top, left + top, left + top),
    [#party-block([FROM], (inv.sender.name,) + inv.sender.address_lines)],
    [#party-block([BILL TO], inv.bill_to_lines)],
    [#party-block([SHIP TO], inv.ship_to_lines)],
  )
]

#v(24pt)
#block(width: 100%, inset: (y: 15pt), stroke: (top: 0.6pt + hairline, bottom: 0.6pt + hairline))[
  #if inv.po_number == "" [
    #grid(
      columns: (1fr, 0.6pt, 1fr),
      column-gutter: 24pt,
      align: (left + top, center, left + top),
      [
        #mono-label([ISSUED])
        #v(4pt)
        #text(font: "JetBrains Mono", size: 10.5pt)[#inv.date_display]
      ],
      [#align(center + horizon)[#vrule]],
      [
        #mono-label([BALANCE DUE])
        #v(4pt)
        #text(font: "JetBrains Mono", size: 12pt, weight: 600)[#inv.balance_due_display]
      ],
    )
  ] else [
    #grid(
      columns: (1fr, 0.6pt, 1fr, 0.6pt, 1fr),
      column-gutter: 24pt,
      align: (left + top, center, left + top, center, left + top),
      [
        #mono-label([ISSUED])
        #v(4pt)
        #text(font: "JetBrains Mono", size: 10.5pt)[#inv.date_display]
      ],
      [#align(center + horizon)[#vrule]],
      [
        #mono-label([PO NUMBER])
        #v(4pt)
        #text(font: "JetBrains Mono", size: 10.5pt)[#inv.po_number]
      ],
      [#align(center + horizon)[#vrule]],
      [
        #mono-label([BALANCE DUE])
        #v(4pt)
        #text(font: "JetBrains Mono", size: 12pt, weight: 600)[#inv.balance_due_display]
      ],
    )
  ]
]

#v(26pt)

#table(
  columns: (1fr, 44pt, 78pt, 94pt),
  column-gutter: 10pt,
  inset: (y: 14pt),
  stroke: (x, y) => if y == 0 { (bottom: 0.6pt + hairline) } else { (bottom: 0.6pt + hairline) },
  align: (left, right, right, right),
  table.header(
    [#mono-label([ITEM])],
    [#align(right)[#mono-label([QTY])]],
    [#align(right)[#mono-label([RATE])]],
    [#align(right)[#mono-label([AMOUNT])]],
  ),
  ..for item in inv.items {
    (
      [#text(weight: 500)[#item.description]],
      [#align(right)[#text(font: "JetBrains Mono", size: 10.5pt)[#item.quantity_display]]],
      [#align(right)[#text(font: "JetBrains Mono", size: 10.5pt)[#item.rate_display]]],
      [#align(right)[#text(font: "JetBrains Mono", size: 10.5pt)[#item.amount_display]]],
    )
  },
)

#v(24pt)

#block(width: 100%)[
  #grid(
    columns: (1fr, 220pt),
    column-gutter: 32pt,
    align: (left + top, right + top),
    [
      #if inv.notes_lines.len() > 0 or inv.tax_note != none [
        #mono-label([NOTES])
        #v(8pt)
        #for line in inv.notes_lines [
          #text(size: 9.4pt, fill: ink-soft)[#line]
          #linebreak()
        ]
        #if inv.tax_note != none [
          #if inv.notes_lines.len() > 0 [#v(8pt)]
          #text(size: 9.4pt, fill: ink-soft)[#inv.tax_note]
        ]
      ]
    ],
    [
      #align(right)[
        #grid(
          columns: (1fr, 120pt),
          column-gutter: 18pt,
          row-gutter: 10pt,
          align: (left + bottom, right + bottom),
          [#text(font: "JetBrains Mono", size: 10.5pt, fill: ink-soft)[Subtotal]], [#text(font: "JetBrains Mono", size: 10.5pt)[#inv.subtotal_display]],
          [#text(font: "JetBrains Mono", size: 10.5pt, fill: ink-soft)[#inv.tax_label]], [#text(font: "JetBrains Mono", size: 10.5pt)[#inv.tax_display]],
          [#box(width: 100%, height: 0.7pt, fill: ink)], [#box(width: 100%, height: 0.7pt, fill: ink)],
          [#text(font: "Instrument Serif", style: "italic", size: 15pt)[Total]],
          [#text(font: "JetBrains Mono", size: 15pt, weight: 500)[#inv.total_display]],
        )
      ]
    ],
  )
]
