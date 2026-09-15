# afm-lint

Adobe Font Metrics (`.afm`) files are still what a lot of PostScript and
PDF tooling reads for glyph widths, kerning, and bounding boxes. They're
plain text and hand-editable, which is exactly why they drift: a build
script regenerates half the file, someone pastes in a merged glyph table
with a duplicate character code, a bad kerning export leaves a negative
width in there. None of that shows up until a PDF renderer produces
garbage spacing, and by then you're staring at a few thousand lines of
`C 32 ; WX 278 ; N space ;` trying to find the one that's wrong.

This is a linter for that file format. It reports problems the way a
compiler would - file, line, column, a caret under the exact span, and
enough context to fix it without opening an editor first.

## Usage

```
cargo run -- Helvetica.afm
```

Given a file with a negative width and a duplicate character code:

```
error[negative-width]: advance width WX -50 is negative
   --> Helvetica.afm:42:11
   |
42 | C 65 ; WX -50 ; N A ;
   |           ^^^ width must be >= 0

error[duplicate-code]: character code 65 already defined on line 42
   --> Helvetica.afm:87:3
   |
87 | C 65 ; WX 722 ; N Aacute ;
   |   ^^

found 2 problems
```

Exit code is 1 if any problems were found, 0 otherwise, so it's usable
as a CI check on generated or vendored font metrics.

## What it checks right now

- required header keys (`FontName`, `FontBBox`, `Ascender`, `Descender`)
  are present before `StartCharMetrics`
- `FontBBox` has exactly four numbers and its lower-left corner is
  actually below and left of its upper-right corner
- `Ascender`/`Descender` parse as numbers, with a note if their sign
  looks backwards (ascender negative, descender positive)
- duplicate character codes (`C`) in the CharMetrics table
- negative advance widths (`WX`)
- lines inside the CharMetrics table that don't parse as
  `key value ; key value ; ...`
- a `StartCharMetrics` count that doesn't match the number of `C` lines
  actually present before `EndCharMetrics`

## What it doesn't do yet

It doesn't touch `KernData` or `Composites` at all. See the roadmap
for where this is headed.

## Format background

AFM is described in Adobe's "Adobe Font Metrics File Format
Specification". Inside `StartCharMetrics N` / `EndCharMetrics`, each
line describes one glyph as semicolon-separated key/value pairs, e.g.
`C 32 ; WX 278 ; N space ;` means character code 32, advance width 278,
glyph name `space`. It predates XML-based metrics formats and is still
what a surprising amount of print and PDF tooling expects as input.
