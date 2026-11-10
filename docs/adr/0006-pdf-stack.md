# ADR-0006: PDF stack

- Status: accepted
- Date: 2026-11-10

## Decision
- **Import:** pdfium-render by default. hayro (pure Rust) runs in a resource-limited worker as an alternative and as the long-term target.
- **Export:** krilla for ink pages, with an invisible text layer. Embedded Typst renders AI summaries with real math.
- **Printing:** the XDG print portal.

## Consequences
- A pdfium binary has to ship, or come from the platform.
- Typst gives print-quality summaries with no LaTeX install.
