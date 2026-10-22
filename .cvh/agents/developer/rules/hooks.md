# Rule: Hooks and Transition Scripts
Refer to `.cvh/rules/hooks.md`. Load via `load_rule("hooks")`.

- Lifecycle hooks: `hooks.on_enter`, `hooks.on_finish`.
- Injected env: `CVH_TASK_ID`, `CVH_TASK_TITLE`, `CVH_WORKTREE_PATH`, `CVH_BRANCH`, `CVH_STAGE_ID`, `CVH_VERDICT`.
- Gatekeeper sensors block transitions on non-zero exit code.
