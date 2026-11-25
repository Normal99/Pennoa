# ADR-0013: Planner, timetable and school platforms

- Status: accepted
- Date: 2026-11-25

## Context
- Students need a timetable, homework and exam dates. Exam dates should drive revision.
- The maintainer's school uses **InSchool (Visma)** plus Teams.
- InSchool has no public student API, no OAuth path and no official student ICS export. Its only official API is OneRoster for school IT.
- Feide gives teaching groups (not timetables), and only after the county activates the service.
- itslearning, iSkole and Canvas offer ICS feeds.

## Decision
- A `pn-planner` crate holds the timetable, homework, tests and exams, linked to subjects. It feeds the Home widgets, the island, reminders, the focus timer and learning paths.
- Input routes, in order:
  1. manual entry, plus AI import from screenshots/PDFs (local VLM first);
  2. ICS/webcal subscriptions;
  3. an **experimental, opt-in, read-only InSchool connector** using the user's own session, labelled unofficial and local-only;
  4. Feide groups and Canvas later, where available.
- Handwritten homework can be lassoed into the planner.

## Consequences
- Works for every student on day one.
- The InSchool connector can break without notice, and may conflict with school or county IT rules, so it is never enabled by default.
