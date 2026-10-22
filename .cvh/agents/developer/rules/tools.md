# Rule: Tools Catalog & Policy Rules
Refer to `.cvh/rules/tools.md`. Load via `load_rule("tools")`.

- Standard tools catalog and ReAct loop dispatch.
- Two-level tool policy (AND-semantics).
- Zero-trust schema stripping.
- English-only schemas invariant for token compression.
- Large output spilling (>500 lines or 64KB -> `.cvh/spill/`).
