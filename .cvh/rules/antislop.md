---
description: Zero-tolerance guidelines for eradicating visual AI slop (gradients, glow dots, glassmorphism) and prose AI slop (throat-clearing, filler phrases, formulaic structures).
---
# Rule: Anti-Slop Discipline for UI and Technical Prose (antislop.md)

Authoritative aesthetic and communication standards for all CanbanVoimaxHarness agents.

---

## 1. Visual UI Anti-Slop Invariants

CanbanVoimaxHarness adheres to a high-density, tactile obsidian control plane aesthetic.
AI slop in user interfaces is strictly eradicated:

1. **No Gratuitous Gradients**:
   - Strictly prohibit indigo→violet, purple→pink, or rainbow gradients.
   - Use flat, solid color tokens from the Antigravity stealth dark palette (`#111114`, `#17171b`, `#232328`, `#212126`, `#2c2c33`, `#3a3a44`, `#f4f4f5`, `#a1a1aa`, `#71717a`, `#52525b`) or Light Paper (`#fafafa`).
   - Legacy dark tokens are banned: `#161b22`, `#21262d`, `#30363d`, `#f0f6fc`, `#8b949e`.
   - Prohibit gradient-clip text headers (`bg-clip-text text-transparent bg-gradient-to-r`).
2. **No Neon Glows & Over-Decoration**:
   - No pulsating neon glow borders (`shadow-[0_0_20px_...]`).
   - No glassmorphism (`backdrop-blur-xl bg-white/5` with noisy borders).
   - No floating background ambient gradient orbs.
   - No wobbling spinners or decorative non-functional animations.
3. **Hierarchy Through Scale, Space & Contrast**:
   - Form follows function: structure content using spacing (`gap`, `padding`), clear typography scale, and subdued border dividers (`border-slate-800`).
   - Reserve bright accent colors strictly for active statuses (amber for changes requested, green for ready, blue/cyan for active tasks).
4. **Monospace Tabular Numerals**:
   - Timers, token counts, TPS, memory metrics, and IDs must always use `font-mono tabular-nums`.
5. **No Decorative Emoji Spam**:
   - Do not litter interface labels, headers, or buttons with decorative emojis. Use canonical Lucide SVG icons.

---

## 2. Technical Prose Anti-Slop Invariants

Technical communication, documentation, and agent reasoning must be dense, direct, and factual. This section is the single owner of prose anti-slop; `anti-sycophancy.md` §2 defers to it and does not restate the banned phrases.

1. **Cut Throat-Clearing & Filler Openers**:
   - Ban conversational fluff and reflexive agreement: "Certainly!", "Yes", "Absolutely", "You're right", "I'd be happy to help", "Let's dive into...", "Here is what we found:".
   - State the action or findings immediately; verify premises against ground truth before any verdict (see `anti-sycophancy.md`).
2. **Break Formulaic AI Structures**:
   - Ban binary contrast crutches: "It's not just X, it's Y", "Not only A, but B". State the direct fact.
   - Ban dramatic fragmentation and artificial one-liner pull quotes.
   - Avoid predictable three-part parallel lists ("speed, reliability, and security").
3. **Active Voice & Concrete Agency**:
   - Use direct active phrasing with concrete subjects.
   - Avoid inanimate abstractions acting like humans ("the logic decides to...").
4. **Direct Coordinate References**:
   - Always reference exact paths (`file:line`) and precise symbol names instead of hand-waving descriptions ("in the relevant database module").
5. **Concise Handover Summaries**:
   - Handover summaries and audit verdicts must prioritize bulleted sensor logs, verifiable file diffs, and exact reproduction steps over prose essays.
