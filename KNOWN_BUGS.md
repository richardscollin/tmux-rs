# Known Bugs

## BUG-001: `paste_get_top` returns borrowed reference instead of owned copy — causes server crash

**Status:** Fixed
**Severity:** Critical (server crash)
**Affected tests:**
- `coverage::set_buffer::set_buffer_rename_top`
- `coverage::window_copy::window_copy_append_selection`
- `coverage::window_copy::window_copy_append_selection_and_cancel`

**Root cause:**

In upstream C, `paste_get_top()` returns the buffer name via `*name = xstrdup(pb->name)` — an owned copy. The Rust translation in `src/paste.rs:118` instead returns a borrow: `*name = Some(&(*pb).name)`.

This causes use-after-free / dangling reference in two places:

1. **`cmd_set_buffer.rs:86`** (`setb -n` rename path): `paste_rename(bufname, ...)` is called with `bufname` borrowing from the paste buffer. Inside `paste_rename`, line 238 overwrites `(*pb).name`, invalidating the `bufname` reference. Line 247 then uses the dangling reference via `oldname.unwrap()`.

2. **`window_copy.rs:5742`** (`append-selection`): `paste_set(buf, len, bufname)` is called with `bufname` borrowing from the paste buffer. Inside `paste_set`, line 284-285 finds and frees the old buffer via `paste_free(old)`, which deallocates the buffer whose name `bufname` borrows from — use-after-free.

**How to reproduce:**
```sh
tmux-rs -f/dev/null new -d -x80 -y24 -L test
tmux-rs -L test setb "data"
tmux-rs -L test setb -n renamed   # server crashes
```

**Fix:** `paste_get_top` should return an owned `String` (or `Option<String>`) instead of borrowing from the paste buffer, matching the upstream `xstrdup` semantics.

---

## BUG-002: Format modifier `#{e|%:10,3}` not implemented

**Status:** Fixed
**Severity:** Low (incorrect output, no crash)
**Affected tests:**
- `regress::format_strings::format_strings`

**Symptom:** `format '#{e|%:10,3}'` returns empty string `''` instead of expected `'1'`. The `e|%` format modifier (modulo operation) may not be implemented or may have a translation error.

---

## BUG-003: `key_bindings_remove_table` frees table before updating client pointers — causes server crash

**Status:** Fixed
**Severity:** Critical (server crash)
**Affected tests:**
- `coverage::unbind_key::unbind_key_all_default_prefix`
- `coverage::unbind_key::unbind_key_all_root`

**Root cause:**

In `src/key_bindings_.rs:301-314`, the function calls `key_bindings_unref_table(table)` **before** iterating clients to clear their `keytable` pointers. If `unref_table` frees the table (refcount drops to 0), the subsequent client loop compares `(*c).keytable` against a freed pointer — use-after-free.

Upstream C (`key-bindings.c:293-307`) does the operations in the correct order:
1. `RB_REMOVE` from tree
2. Iterate clients and fix `keytable` pointers
3. `key_bindings_unref_table` (free last)

The Rust version swaps steps 2 and 3.

**How to reproduce:**
```sh
tmux-rs -f/dev/null new -d -x80 -y24 -L test
tmux-rs -L test unbind-key -a    # server crashes
```

**Fix:** Move `key_bindings_unref_table(table)` after the client loop, matching upstream order.

---

## BUG-004: `send-prefix -2` crashes the server when prefix2 is None

**Status:** Fixed
**Severity:** Critical (server crash)
**Affected tests:**
- `coverage::send_keys::send_prefix_2`

**Root cause:**

`send-prefix -2` calls `options_get_number___::<u64>(&*(*s).options, "prefix2")` in `src/cmd_/cmd_send_keys.rs:210`. When `prefix2` is set to `None` (the default), this likely panics or returns an invalid value causing a crash.

In upstream C, `options_get_number(s->options, "prefix2")` returns the default value `KEYC_NONE` (0), which is harmlessly passed to `cmd_send_keys_inject_key`.

**How to reproduce:**
```sh
tmux-rs -f/dev/null new -d -x80 -y24 -L test
tmux-rs -L test send-prefix -2    # server crashes
```

**Fix:** `options_get_number___` should return `KEYC_NONE` (0) when the option value is `None`, matching upstream behavior.

---

## BUG-005: `split-window -f` crashes the server

**Status:** Fixed
**Severity:** Critical (server crash)
**Affected tests:**
- `coverage::split_window::split_window_fullsize`
- `coverage::split_window::split_window_fullsize_horizontal`

**Root cause:** TBD — the `-f` (fullsize) flag causes a crash when splitting with multiple panes. The crash occurs in `layout_split_pane` or related layout code when `SPAWN_FULLSIZE` is set.

**How to reproduce:**
```sh
tmux-rs -f/dev/null new -d -x80 -y24 -L test
tmux-rs -L test split-window -d
tmux-rs -L test split-window -f -l 5    # server crashes
```

**Fix:** Needs investigation — compare `layout_split_pane` with upstream when `SPAWN_FULLSIZE` flag is set.

---

## BUG-006: `select-layout even-horizontal` crashes the server with 3+ panes

**Status:** Fixed
**Severity:** Critical (server crash)
**Affected tests:**
- `coverage::layout_set::layout_even_horizontal_3panes`
- `coverage::layout_set::layout_cycle_next_prev` (hits even-horizontal during cycle)
- `coverage::layout_set::layout_set_with_tiny_window` (hits even-horizontal)

**Root cause:** TBD — the `layout_set_even_horz` function in `layout_set.rs` crashes when arranging 3+ panes. Other layout presets (`even-vertical`, `main-horizontal`, `main-vertical`, `tiled`) work correctly.

**How to reproduce:**
```sh
tmux-rs -f/dev/null new -d -x80 -y24 -L test
tmux-rs -L test split-window -d
tmux-rs -L test split-window -d
tmux-rs -L test select-layout even-horizontal    # server crashes
```

**Fix:** Compare `layout_set_even_horz` with upstream `layout_set.c`.

---

## BUG-007: Regex substitution backreference `\1` appends extra character

**Status:** Fixed
**Severity:** Medium (incorrect output)
**Affected tests:**
- `coverage::regsub::regsub_backreference`

**Root cause:** In `src/regsub.rs`, when processing backreferences like `\1`, the replacement copies one extra byte. For input `foo123bar` with pattern `([0-9]+)` and replacement `[\1]`, tmux-rs produces `foo[1231]bar` instead of the correct `foo[123]bar` (which upstream tmux returns).

**How to reproduce:**
```sh
tmux-rs -f/dev/null new -d -x80 -y24 -L test
tmux-rs -L test set -g @v "foo123bar"
tmux-rs -L test display -p '#{s/([0-9]+)/[\1]:#{@v}}'
# Returns: foo[1231]bar (should be: foo[123]bar)
```

**Fix:** Check the `regsub_copy` call for backreferences in `regsub.rs` — likely an off-by-one in the `rm_so`/`rm_eo` range calculation.

---

## BUG-008: `#{window_layout}` crashes the server with mixed horizontal/vertical splits

**Status:** Fixed
**Severity:** Critical (server crash)
**Affected tests:**
- `coverage::layout_custom::layout_custom_mixed`

**Root cause:** `layout_dump` (or format expansion of `#{window_layout}`) crashes when the layout contains both vertical and horizontal splits (e.g., 2 panes split vertically, then one sub-pane split horizontally).

**How to reproduce:**
```sh
tmux-rs -f/dev/null new -d -x80 -y24 -L test
tmux-rs -L test split-window -d
tmux-rs -L test split-window -d -h -t :.1
tmux-rs -L test display -p '#{window_layout}'    # server crashes
```

Note: `#{window_panes}` works correctly (returns 3). Only `#{window_layout}` crashes.

**Fix:** Compare `layout_dump` with upstream `layout.c`.
