# Rule: UI Visual Cleanliness & Technical Prose Standards (antislop.md)

Authoritative guidelines uniting visual cleanliness standards (the 35-tell AI-Slop taxonomy) and strict technical prose discipline across CanbanVoimaxHarness.

---

## 1. Visual UI Invariants (UI Anti-Slop / kill-ai-slop)

### 1.1 Color Palette & Restrained Accents
- **Ban on Gradient Clutter**: Strictly forbid multi-color gradient transitions (e.g., `#6366f1` to `#a855f7`), rainbow border wrappers, and atmospheric blur blobs (`blur-3xl`).
- **Single Dominant Accent**: Each view utilizes exactly one semantic accent color for primary interactive triggers.
- **High-Contrast Solid Text**: Forbid gradient text (`bg-clip-text text-transparent`). Render all text with high-contrast solids: `#171717` / `#4d4d4d` in Light Paper mode and `#f0f6fc` / `#8b949e` in Obsidian Dark mode.

### 1.2 Geometry & Layout Density
- **Restrained Corner Radii**: Use standard radii `rounded-md` (6px) or `rounded-lg` (8px). Forbid exaggerated `rounded-3xl` bubble cards and excessive `backdrop-blur`.
- **Crisp Hairline Dividers**: Separate blocks with subtle 1px borders (`#ebebeb` / `#d4d4d8` for Light mode, `#21262d` / `#30363d` for Dark mode). Heavy shadows (`shadow-2xl`) are permitted exclusively on floating context menus and modal overlays.
- **Monospace Telemetry Figures**: Apply `.tabular-nums` across all timers, token counts, TPS metrics, and duration pills to eliminate layout jitter.

### 1.3 Indicators & Iconography
- **No Tinted Background Squares on Icons**: Render Lucide vectors directly as clean monochrome strokes without pastel square chips (`bg-blue-100`) behind them.
- **Ban on Badge Inflation**: Never wrap plain text in decorative pills ("✨ New", "🔥 Popular"). Use badges exclusively for binary execution statuses or audit verdicts.
- **No Emojis in Controls**: Column headers, action buttons, tool descriptions, and task cards must never contain emojis (🚀, 💡, 🔥, ✨).

---

## 2. Technical Prose & Documentation Invariants (Prose Anti-Slop)

### 2.1 Active Voice & Concrete Actors
- Attribute every action to a concrete subject: "Backend commits the transaction", "Reviewer rejects the feature branch".
- Never personify inanimate concepts: avoid phrases like "the code wants to be clean" or "the logic decided".

### 2.2 Banned Filler & Fluff
- Forbid conversational throat-clearing: "It is worth noting that", "Needless to say", "In today's fast-paced world".
- Forbid synthetic contrasts: "It is not just a tool, it is an ecosystem". State the operational fact directly: "The engine dispatches Kanban tasks".
- Forbid marketing buzzwords: `game changer`, `paradigm shift`, `streamline`, `delve`, `foster`, `leverage`, `robust`, `supercharge`, `tapestry`, `holistic`.

### 2.3 Report & Commit Message Structure
- **Direct Fact-First Delivery**: The first sentence of any commit or handover summary must name the exact technical modification and affected component.
- **No False Grandeur**: Cease writing rhetorical conclusions, philosophical summaries, or congratulatory sign-offs.
- **Portability Test**: Any sentence that could be copied into an unrelated project without losing technical meaning must be pruned immediately.
