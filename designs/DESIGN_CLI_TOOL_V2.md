# Sashiko CLI Tool Design V2 (UX Refinement)

## Objective
Provide a "DWIM" (Do What I Mean) CLI experience. Minimizing required arguments while maintaining clarity and safety.

## Global Options
- `--server`: Env var `SASHIKO_SERVER` or settings.
- `--format`: `text` (default) or `json`.

## Commands & Improvements

### 1. `submit`
**Goal:** Submit a patch, mbox, or range with minimal friction.

**Current:**
`sashiko submit [REVISION] --type <TYPE> --path <PATH>`

**Improved UX:**
- `sashiko submit` (no args):
  - Detects if CWD is a git repo.
  - Defaults to `HEAD` if inside a repo.
  - Defaults to `.` if standard input is piped (mbox).
- `sashiko submit <INPUT>`:
  - If `<INPUT>` is a file path -> `mbox` type.
  - If `<INPUT>` looks like a range (`..`) -> `range` type.
  - If `<INPUT>` looks like a SHA/ref -> `remote` type (or `range` if it implies `HEAD`).
  - If `<INPUT>` is `-` -> stdin mbox.
- `sashiko submit .`:
  - Implies `HEAD` of current repo? Or `mbox` from current dir?
  - `git` usually uses `.` for "current path".
  - If CWD is a git repo, `sashiko submit` should probably submit `HEAD`.

**Revised Signature:**
```bash
sashiko submit [INPUT] [OPTIONS]
```
- `[INPUT]`: File path, Commit SHA, Range, or `-`.
  - Default: `HEAD` (if in git repo), else Error (or help).
- `--repo <PATH>`: Override repository path (defaults to CWD if it's a repo, else configured default).
- `--baseline <COMMIT>`: Only for mbox.

**DWIM Logic:**
1. Is stdin piped? -> `Mbox` (read from stdin).
2. Is `INPUT` a file? -> `Mbox` (read file).
3. Is `INPUT` a range (`a..b`)? -> `Range` (repo = CWD or default).
4. Is `INPUT` a commit (`sha`)? -> `Remote` (repo = CWD or default).
5. No `INPUT`? -> `Remote` (`HEAD`, repo = CWD).

### 2. `status`
**Goal:** Quick system check.
- `sashiko status`: Shows queue stats.
- **Improvement:** Add `--watch` `-w` to keep updating? (Maybe later).
- **Improvement:** Show "Server is offline" friendly error if connection fails.

### 3. `list`
**Goal:** Find what's happening.
- `sashiko list`: Shows recent patchsets (default page 1).
- **Improvement:**
  - Aliases: `ls`.
  - Filter shortcuts: `sashiko list pending`, `sashiko list failed`.
  - Search: `sashiko list "search term"`.
  - Combined: `sashiko list pending "bpf"`.

### 4. `show`
**Goal:** view details.
- `sashiko show <ID>`.
- **Improvement:**
  - Auto-detect if ID is not provided? (Maybe last submitted?).
  - Allow `HEAD` if we can resolve it to a patchset ID? (Complex).
  - Show "most recent" if no ID? `sashiko show latest`.
  - Add `--watch` `-w` flag to stream status updates linearly.
  - Parity between JSON and text outputs (show detailed findings in text format).

### 5. `rerun`
**Goal:** Request a re-review of a completed patchset.
- `sashiko rerun <ID>`.
- **Improvement:**
  - Placed back into `Pending` queue.

### 6. `local`
**Goal:** Run a local review with optional interactive loop.
- `sashiko local [INPUT] [OPTIONS]`.
- **Improvement:**
  - Add `--interactive` flag.
  - If issues are found, pauses and prompts the user/agent to modify the code or provide a rebuttal.
  - Loops until review is clean (LGTM).

## Action Plan
1.  **Refine `submit` argument parsing:** Implement robust detection logic.
2.  **Add `list` shortcuts:** Allow positional arguments for status filter.
3.  **Add `show` aliases:** Support `latest`.
4.  **Formatting:** Ensure consistent, colorful output.
