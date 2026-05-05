# Design: Subject-Aware Embargo Policy Selection

## Problem Statement
Currently, when a patch is cross-posted to multiple mailing lists, the system uses the shortest explicitly configured embargo period (`min()`). However, closely related subsystems often cross-post. For example, a patch targeting `net-next` might be sent to `netdev@vger.kernel.org` (24h embargo) and CC'd to `bpf@vger.kernel.org` (0h embargo). Because of the `min()` rule, the patch incorrectly receives a 0-hour embargo, circumventing the intended 24-hour review period for the `net` subsystem.

## Proposed Solution
We will enhance `calculate_embargo_hours` to be "subject-aware". By inspecting the patch subject prefix (e.g., `[PATCH net-next ...]`), the system can determine the primary intended subsystem and prioritize its embargo policy over the fallback `min()` behavior.

### 1. Configuration Changes (`email_policy.toml` and `src/email_policy.rs`)
Add a new optional list `subject_prefixes` to the `SubsystemPolicy` struct. This allows administrators to explicitly map Git tree prefixes to their respective subsystem.

```toml
[subsystems.net]
lists = ["netdev@vger.kernel.org"]
embargo_hours = 24
subject_prefixes = ["net", "net-next", "netdev"]

[subsystems.bpf]
lists = ["bpf@vger.kernel.org"]
embargo_hours = 0
subject_prefixes = ["bpf", "bpf-next"]
```

### 2. Subject Parsing
Implement a lightweight regex or string parser in `src/patch.rs` (or directly in `main.rs`) to extract the text between `[` and `]` in the subject line, and isolate the tree name (ignoring `PATCH`, `RFC`, `v2`, `n/m`, etc.).
- `[PATCH net-next v3 07/13] ...` -> `net-next`
- `[RFC PATCH bpf-next] ...` -> `bpf-next`
- `[PATCH v2 bpf 0/6] ...` -> `bpf`

### 3. Updated Logic in `calculate_embargo_hours`
Update the function signature to accept the patch `subject: &str`.
The new evaluation priority will be:
1. Identify all subsystems matched by the `To`/`Cc` email addresses (same as today).
2. Extract the prefix from the `subject`.
3. Check if any of the **matched** subsystems contain this prefix in their `subject_prefixes` configuration.
    - If **YES**: Use the `embargo_hours` of that specific subsystem, ignoring the others.
    - If **NO**: Fall back to the existing behavior: return the minimum `embargo_hours` among all matched subsystems.

## Implementation Steps
1. Update `SubsystemPolicy` in `src/email_policy.rs` to include `subject_prefixes: Vec<String>` with `#[serde(default)]`.
2. Write a helper function `extract_subject_prefix(subject: &str) -> Option<String>` and add unit tests for it.
3. Update `calculate_embargo_hours` in `src/main.rs` to use this new logic.
4. Add unit tests for `calculate_embargo_hours` simulating the `net-next` vs `bpf` cross-posting scenario.
