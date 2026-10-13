# CanbanVoimaxHarness — Style Architecture & Design Philosophy
> Synthesis of Vercel ("Typeset terminal on white paper / obsidian") & ChatGPT ("Graphite ink on warm paper")

This directory codifies the visual and tactile design system for **CanbanVoimaxHarness**. The user interface is engineered as an ultra-fast, high-density desktop control plane combining the restrained utility of **ChatGPT** (graphite-on-paper, spatial hierarchy, zero visual clutter) with the typographical and mechanical discipline of **Vercel** (monospace CLI stamping, hairline borders, 6px radii, the ▲ brand protocol).

---

## 1. Core Principles & Zero-Slop Standard

1. **Disciplined Monochrome (0% Colorfulness in Chrome)**:
   - Interface chrome is completely achromatic. Meaning is conveyed through typographic weight, tonal value contrast, and spatial cadence, never decorative gradients or colored panels.
   - Primary surfaces stack cleanly:
     - **Canvas**: Paper White (`#fafafa`) / Sidebar Mist (`#f9f9f9`)
     - **Elevated Surfaces & Cards**: Pure White (`#ffffff`)
     - **Inverted Panels & Terminal**: Obsidian (`#171717`) / Deep Slate
     - **Graphic Marks & Icons**: Carbon (`#000000`) / Pure White (`#ffffff`)
     - **Hairline Borders**: 1px Hairline (`#ebebeb` or `rgba(0,0,0,0.08)` / dark: `#27272a`)
     - **Graphite & Secondary Ink**: `#0d0d0d` (Graphite Ink), `#4d4d4d` (Charcoal), `#8f8f8f` (Graphite/Hollow)

2. **The Sole Chromatic Accent — Terminal Green (`#297a3a`)**:
   - The only allowed accent color in status indicators is Terminal Green (`#297a3a`), reserved exclusively for positive verification and ready states (`✓ READY`, `[SYSTEM READY]`).
   - All other state shifts rely on weight, hairline contrast, and positional signaling.

3. **Hairline Elevation vs. Zero Drop-Shadows**:
   - Elevation is expressed solely through 1px hairline boundaries (stacked box-shadow rings `0 0 0 1px rgba(0,0,0,0.08)` or clean 1px solid borders `#ebebeb`).
   - Heavy drop-shadows and fuzzy colored glows are strictly eliminated.

4. **Geometry & Radii**:
   - Strict 6px border radii (`--geist-radius`, `rounded-[6px]` / `rounded-md`) across all cards, panels, inputs, and buttons.
   - Full pills (`rounded-full`) are permitted only for specialized top-level status badges and navigation pills.

5. **Typographic Duality**:
   - **Geist Sans / System UI**: Used for reading, navigation, and modal dialogues at weights 400 and 450 (tight tracking `-0.05em` on display headers). Confident without shouting.
   - **Geist Mono / JetBrains Mono**: Mechanical stamping of CLI outputs, version stamps, status tags, token counters, and telemetry readouts.

---

## 2. Directory Contents

- [vercel.md](./vercel.md) — Full specification of the Vercel design system ("Typeset terminal on white paper", Geist type system, hairline rings, ▲ triangle protocol).
- [chatgpt.md](./chatgpt.md) — Full specification of the ChatGPT design system ("Graphite ink on warm paper", achromatic spatial rhythm, hairline borders, compact density).
