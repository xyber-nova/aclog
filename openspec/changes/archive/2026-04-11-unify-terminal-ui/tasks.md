## 1. Terminal Module Skeleton

- [x] 1.1 Create `src/ui/terminal/` with `mod.rs`, `theme.rs`, `common.rs`, `sync.rs`, `selector.rs`, `browser.rs`, and `stats.rs`
- [x] 1.2 Move terminal bootstrap logic such as alternate-screen/raw-mode setup into `src/ui/terminal/mod.rs`
- [x] 1.3 Convert `src/tui.rs` into a thin compatibility facade that forwards to `src/ui::terminal`
- [x] 1.4 Update `src/ui/interaction.rs` so `TerminalUi` dispatches to `crate::ui::terminal::*` instead of owning direct `crate::tui::*` calls

## 2. Shared Theme And Interaction Helpers

- [x] 2.1 Implement shared semantic styles in `theme.rs` for verdicts, warnings, invalid states, accents, hints, and selected rows
- [x] 2.2 Add shared layout and rendering helpers in `common.rs` for headers, footers, empty states, summary panes, and help panels
- [x] 2.3 Extract shared key-handling helpers for up/down navigation, `j/k` aliases, and `?` help toggling

## 3. Selector And Sync UI Migration

- [x] 3.1 Rebuild record bind/rebind submission selectors with the shared “context + table + detail + actions” layout
- [x] 3.2 Rebuild sync preview to show the batch table alongside a live summary pane for the selected item
- [x] 3.3 Rebuild sync detail and delete-confirmation pages to show problem/file context, warnings, empty states, and available actions in distinct regions
- [x] 3.4 Preserve existing command semantics and primary key bindings while adding `j/k` and `?` support to selector and sync screens

## 4. Browser And Stats UI Migration

- [x] 4.1 Rebuild record browser root views and timelines with consistent left-list/right-detail structure and explicit filter/view summaries
- [x] 4.2 Rebuild stats overview to use grouped summary sections plus shared themed distribution panels
- [x] 4.3 Rebuild review mode to be visually distinct from overview while keeping compatible drill-down behavior into browser views
- [x] 4.4 Add shared help and action hints to browser and stats screens without changing their existing workflow semantics

## 5. Tests And Verification

- [x] 5.1 Move existing pure TUI tests out of the old monolithic module into the new terminal submodules
- [x] 5.2 Add tests for semantic style mapping, including verdict colors and warning/invalid status styling
- [x] 5.3 Add tests for shared key handling so `j/k` matches arrow navigation and `?` only toggles help state
- [x] 5.4 Add tests for summary/empty-state/filter-summary rendering helpers used by sync, selectors, browser, and stats
- [x] 5.5 Run `cargo fmt --check`, `cargo check`, and `cargo test`
