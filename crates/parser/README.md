# parser

Reads a Tatra banka PDF statement into a `Statement`. `pdfium.rs` extracts
text with PDFium; `lines.rs` turns PDFium's per-character geometry into text
lines; `header.rs`, `blocks.rs`, `fields.rs`, `card.rs`, `transfer.rs` read
the fields out of those lines.

## Two spacing paths in `lines::group_lines`

PDFium gives every character a loose box (its full advance) and a tight box
(the ink only). Older statements report a real gap between the two: the
loose box is the character's true advance width, so dividing the gap between
boxes by that width gives the number of spaces. A generator seen on the
2026-06-30 statement instead reports identical loose and tight boxes for
every glyph, so a glyph's own width says nothing about spacing (a narrow `1`
is still one monospace cell wide).

`Char::advance` holds the loose width only when loose and tight measurably
differ (checked on width AND height, since a wide letter like `A` can have
equal widths on a real advance-metric font while its height still carries
the full line-height padding). A row uses the old, unchanged rule
(`spaced_by_advance`) only when every glyph in it has `advance: Some`.
Otherwise (`spaced_by_origin`) it falls back to the origin-to-origin pitch:
the median of the row's smallest consecutive deltas is the monospace cell
width, and a gap past 1.5 cells becomes `round(delta / unit) - 1` spaces.

Row membership (`group_lines`) also switched from the ink box's bottom edge
to the text baseline (`Char.y`, PDFium's origin y), with the threshold taken
from the font size (half of it) instead of half the ink height: a descender
like `ý` sits lower in its own box but shares its baseline with the rest of
the row.

PDFium also synthesises extra whitespace characters on some generators: a
real `' '` codepoint with a degenerate zero-size box, at an x position that
does not sit between the words it separates. `group_lines` drops these
before doing anything else, so every space in the output comes from measured
glyph geometry, never from a character PDFium invented.

`abakus-cli lines <pdf>` and `abakus-cli geometry <pdf>` are diagnostics for
this: `lines` prints the masked, grouped text; `geometry` prints PDFium's
raw per-character boxes, origin and font for page 1.
