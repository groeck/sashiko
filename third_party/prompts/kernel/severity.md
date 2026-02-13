# Severity Levels

When identifying issues, you must assign a severity level to each finding. Use the following definitions:

## Critical
- **Definition**: Issues that cause data loss, memory corruptions or security vulnerabilities.
- **Action**: Must be fixed immediately. The patch cannot be merged.
- **Examples**:
    - Security vulnerability (e.g., buffer overflow, use-after-free).
    - Data corruption.
    - ABI breakage without proper deprecation.
	- Memory corruptions
	- Kernel panic or oops which can be triggered externally.

## High
- **Definition**: Serious issues that affect functionality and performance.
- **Action**: Should be fixed before merging or fixed ASAP.
- **Examples**:
    - Rare kernel panic or oops.
    - Logic errors leading to incorrect behavior.
    - Significant performance regression.
    - Resource leaks (memory, locks).
    - Violation of core kernel locking rules.
    - Incorrect error handling.

## Medium
- **Definition**: Issues that improve code quality, readability, or minor functional improvements.
- **Action**: Recommended to be fixed. Can be addressed in a follow-up patch or v2.
- **Examples**:
    - Build failures.
    - Coding style violations (checkpatch warnings).
    - Confusing variable naming or comments.
    - Minor efficiency improvements.
    - Unnecessary code complexity.
    - Missing documentation for new APIs.

## Low
- **Definition**: Nitpicks, suggestions, or questions.
- **Action**: Optional fixes.
- **Examples**:
    - Typos in comments.
    - Formatting suggestions (whitespace).
    - Personal preference on code structure.
    - Clarification questions.
