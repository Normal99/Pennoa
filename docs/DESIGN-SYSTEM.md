# Design system: "Calm Aero"

The interactive prototype lives in [`design/prototype/`](../design/prototype/) and is published as a private claude.ai canvas. These are its source of truth: the tokens below map 1:1 onto Slint globals.

## Direction
We take **structure and motion** from ColorOS 17 (OnePlus 16) and **reject its liquid glass**.

| Adopt | Reject |
|---|---|
| Elastic, springy feedback on press and transitions | Refraction, lensing, "liquid" distortion |
| A **floating island** for live status (sync, recognition, AI streaming, focus timer) | Specular highlights, contour glow |
| Floating navigation and tool bars | Glass behind content, or glass on cards and sheets |
| Fully circular toggles | Translucency as decoration |
| Big rounded cards, bold large titles, widget tiles | — |

**Material rule**
- **Solid by default.** Cards, sheets, settings and pages are opaque.
- **Only floating overlays above the canvas** use the *Calm Aero float*: the pen toolbar, the island, the selection capsule, and the zoom and page pills.
- The float is 84% surface opacity, 20 px blur with 1.15 saturation, a 1 px light inner top edge, a 1 px outer ring and a soft shadow. It is a light Windows-Aero touch: crisp and readable, never lensing.
- Settings → Appearance → "Frosted floating tools" switches to a fully opaque float, for battery and weak iGPUs.

## Tokens

### Colour
| Token | Light | Dark |
|---|---|---|
| `ground` | `#F3F4F7` | `#0F1115` |
| `surface` | `#FFFFFF` | `#181B21` |
| `surface-2` | `#F3F5F8` | `#21252D` |
| `line` | `#E3E6EC` | `#2C313B` |
| `text` | `#14161B` | `#F1F3F6` |
| `text-2` | `#454B57` | `#C3C9D3` |
| `text-3` (min. for small text) | `#5F6672` | `#99A1AE` |
| `accent` | `#2557D6` | `#7FA2FF` |
| `accent-ink` (on accent) | `#FFFFFF` | `#0B1020` |
| `accent-tint` | `#E8EEFC` | `#1C2740` |
| `ok` | `#1C7D45` | `#5BD08E` |
| `float` | `rgba(255,255,255,.84)` | `rgba(30,33,40,.86)` |
| `ink` (default pen) | `#1D2B5C` | `#C9D6FF` |

**Subject accents** are shown as a 15% tint with full-colour glyphs:

| Subject | Colour |
|---|---|
| Mathematics | `#2F64E0` |
| Physics | `#C2620A` |
| Chemistry | `#0F8F86` |
| Biology | `#2E8B3E` |
| History | `#7C4DDB` |
| Norsk | `#D63B73` |
| English | `#4B57D6` |

Users can choose any colour; contrast is checked automatically. Every text pairing meets WCAG AA (4.5:1) in both themes.

### Type
- **Font:** Inter (OFL), which covers Latin (incl. æ ø å), Greek and Cyrillic. The Noto family covers every other script. OPPO Sans and other OPPO assets are **not** used.
- **Languages:** English and Norwegian (nb, nn) ship first. Layouts must allow about 30% longer strings and mirror for RTL.
- **Math:** STIX Two.
- **Your handwriting:** rendered from your strokes, never imitated with a font. The prototype uses Caveat only as a stand-in.

| Role | Size / weight | Tracking |
|---|---|---|
| Display | 44 / 800 | −0.03em |
| Title | 32 / 800 | −0.03em |
| Headline | 20 / 700 | −0.01em |
| Body | 15 / 400 | 0 |
| Label | 13 / 600 | 0 |
| Caption | 12 / 400–600 | 0 |

### Shape, spacing, elevation
- **Radii:** chip 8, tile 16, card 24, sheet 28, pill 999.
- **Spacing:** a 4 pt grid (4, 8, 12, 16, 24, 32, 48).
- **Targets:** touch targets ≥ 44 px; pen tool buttons 48 px.
- **Elevation:** `card` = 0 1 2 / 5%; `float` = 0 10 30 / 12% + 0 2 6 / 6%; dark mode uses stronger shadows and no card shadow.

### Motion
| Token | Spec |
|---|---|
| `press` | scale 0.95; in 90 ms; out 460 ms `cubic-bezier(.34,1.56,.64,1)` |
| `island` | size 520 ms `cubic-bezier(.34,1.4,.64,1)`; content cross-fade 160 ms |
| `pop` (capsule, chips) | scale 0.6 → 1 over 520 ms with overshoot; exit 140 ms ease-in |
| `toggle` | knob 420 ms `cubic-bezier(.34,1.56,.64,1)`; track colour 200 ms |
| Reduced motion | All springs become 120 ms ease-out with no overshoot |

**Slint mapping** (example):
```slint
export global Theme {
    in-out property <bool> dark;
    out property <color> surface: dark ? #181B21 : #FFFFFF;
    out property <length> r-card: 24px;
    out property <duration> press-out: 460ms;
    out property <easing> spring: cubic-bezier(0.34, 1.56, 0.64, 1);
}
// usage: animate scale { duration: Theme.press-out; easing: Theme.spring; }
```

## Components
- **Island:** a top-centre capsule (40 px) that expands into a rounded rectangle (radius 28) on tap. It has one primary state at a time, in priority order: focus timer > AI streaming > recognition > sync.
- **Floating pen toolbar:** at the bottom centre over the canvas. Tools are 48 px circles; the active tool is inverted (text colour on surface). It holds 4 ink swatches, undo and redo.
- **Selection capsule:** pops above the lasso. It holds Cut · Copy · Paste · To text · Ask AI · **Calculate** (primary) · Flashcard · ×.
- **Autocalc chip:** shows the result, a "CAS-verified" badge, Insert and Explain steps.
- **Ask-AI sheet:** a solid right sheet (448 px, radius 28) with context chips, a streamed answer, citations, quick actions, and an input box with a "write by hand" button.
- **Circular toggle:** a 52×32 track with a 26 px knob and a springy knob animation.
- **Cards:** radius 24, solid, with a subtle shadow in light mode only.

## Screens (prototype artboards)
1. Home / Library
2. Notebook editor, with the selection capsule and autocalculate
3. Ask-AI side sheet (dark)
4. Scanner (dark)
5. Learning path, with the focus timer in the island
6. Export to PDF
7. Settings: sync, AI providers, recognition, pen, phone, appearance
8. Design-system sheet

The layout targets a 1280×800 landscape tablet posture (Yoga). Laptop posture uses the same layout with keyboard shortcuts. Narrow windows stack the side sheets as full-height overlays.
