# ADR-0005: Autocalculate

- Status: accepted
- Date: 2026-11-10

## Decision
- Pipeline: recognised LaTeX → own parser → AST.
  - Numeric and unit work goes to **fend-core** (MIT).
  - Symbolic work goes to **Giac** (GPL-3+) through our own `cxx` shim.
- Only results from these engines get a "verified" badge. An LLM may explain a result, but never certifies it.
- Symbolica is rejected: it is source-available and incompatible.

## Consequences
- We build and package Giac.
- The parser supports a well-defined LaTeX subset and refuses anything it doesn't understand, rather than guessing.
