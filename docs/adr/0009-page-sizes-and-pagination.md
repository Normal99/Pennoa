# ADR-0009: Page sizes, infinite canvas and pagination

- Status: accepted
- Date: 2026-11-10

## Context
- A4 is the default, but students also want larger, smaller or unbounded space, like OneNote's infinite canvas.
- Printing and PDF export still need real pages.

## Decision
- **Paper** is a page property: A5, A4, A3, Letter, Legal, custom W×H (portrait or landscape), or **infinite**. Changing it never modifies the ink.
- **Export** runs the `si-layout` pagination engine:
  1. cluster content into atomic blocks;
  2. sort them into reading order;
  3. fit to the page width (≥ 70% scale by default);
  4. flow the blocks onto pages, never cutting a drawing or formula.
- **Manual layout:** blocks can be dragged between pages and page breaks can be moved. Manual moves become pins, which auto layout respects.
- The layout is stored non-destructively as `print_layout` in the notebook and synced.

## Consequences
- One engine serves infinite → A4 export and any size → size re-pagination.
- Block clustering quality matters, so it reuses the recognition regions and is covered by a layout test set.
