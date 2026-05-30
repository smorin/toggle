# toggle

A Rust CLI for toggling comment blocks in source code. Comment / uncomment line
ranges, named sections, or grouped variants across one or many files —
deterministic, atomic, language-aware.

## Install

```bash
cargo install --path .
# or, from crates.io:
cargo install togl
```

Pre-built binaries via Homebrew are not yet published; see the [Distribution](#distribution) section.

## Quick start

```bash
# Toggle a line range in a Python file
toggle -l 10:20 main.py

# Toggle a named section across all matching files
toggle -S featureXYZ src/

# Force a section commented across a tree
toggle -S debug --force on -R src/

# Discover what sections exist in a tree
toggle --scan -R src/
```

## Subcommands

Every operation is also available as a subcommand that exposes only the flags
relevant to it. The subcommands are equivalent to the flat flags below (they run
through the same engine), but are easier to discover and harder to misuse:

| Subcommand | Flat-flag equivalent |
|---|---|
| `toggle <paths> -S id` | `toggle <paths> -S id` |
| `toggle scan -R src/` | `toggle --scan -R src/` |
| `toggle check -R src/` | `toggle --scan --check -R src/` |
| `toggle list src/` | `toggle --list-sections src/` |
| `toggle insert main.py -S id -l 10:20` | `toggle --insert -S id -l 10:20 main.py` |
| `toggle remove main.py -S id` | `toggle --remove -S id main.py` |

Run `toggle <subcommand> --help` to see its scoped flags. The flat-flag form
still works and is supported, but is deprecated in favor of the subcommands.

## Section markers

Wrap any block in a paired marker comment that the tool can find:

```python
# toggle:start ID=featureX desc="Optional description"
print("guarded code")
# toggle:end ID=featureX
```

The single-line comment style is inferred from the file extension; override
with `--comment-style "//"` (single) or `--comment-style "//" "/*" "*/"` (with
multi-line delimiters).

## Section variants (`group:variant`)

Use a `:` in the ID to mark variants of the same group. The CLI then knows
how to swap, activate, or fan-out across them.

```python
# toggle:start ID=db:sqlite
import sqlite3
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres
# import psycopg2
# toggle:end ID=db:postgres
```

| Command | Behavior |
|---|---|
| `toggle -S db file.py` | **Pair flip** — swap active and commented variants (errors on 3+ variants without a qualifier) |
| `toggle -S db:postgres file.py` | **Activate** — uncomment `db:postgres`, comment every other `db:*` |
| `toggle -S db --force on file.py` | **Force all** — comment every variant in `db` |
| `toggle -S db --pair file.py` | **Guard** — fail before any write if `db` does not have exactly 2 variants |

## Inserting a section

Wrap an existing line range in marker comments without commenting the body:

```bash
# Wrap lines 10–20 of main.py in an ID=featureX marker pair
toggle --insert -S featureX -l 10:20 main.py

# With a description
toggle --insert -S featureX -l 10:20 --desc "new feature" main.py
```

`--insert` operates on a single file and leaves the body uncommented. Run
`toggle -S featureX main.py` afterward to comment the block.

## Scan & check

```bash
# Per-file table with a TYPE column (solo / pair / group)
toggle --scan src/app.py

# Recursive summary, one row per group
toggle --scan -R src/

# Detailed view of one group: file refs + state per variant
toggle --scan -S db -R src/

# Validate without modifying: unclosed markers, duplicate IDs, cross-file gaps
toggle --scan --check -R src/

# Same, but only flag groups that should be pairs
toggle --scan --check --pair -R src/

# Machine-readable nested JSON
toggle --scan -R src/ --json
```

`--check` exits non-zero on any error finding (unclosed markers, duplicate
IDs); warnings (variant gaps, pair-count mismatches) do not fail the run.

## Filter mode (stdin → stdout)

The writer operations (`toggle`, `insert`, `remove`) can read from stdin and
write the transformed result to stdout, leaving any file untouched — so `toggle`
composes in a pipeline. Use a `-` path, or the `--stdin` / `--stdout` aliases:

```bash
# Toggle a section, reading stdin and writing stdout
cat main.py | toggle - -S featureX

# Equivalent spellings
toggle --stdin  -S featureX < main.py
toggle --stdout -S featureX < main.py

# Works with the subcommands too
toggle remove --stdin -S featureX < main.py
toggle insert --stdin -S featureX -l 10:20 < main.py > wrapped.py
```

Piped input has no file extension, so the comment style defaults to `#`
(Python); pass `--comment-style` for other languages. Filter mode is a single
stdin→stdout transform: it does not accept file paths, `--json`, `--atomic`,
`--backup`, `--dry-run`, `--interactive`, or `-R`.

## Atomic multi-file mode

```bash
# All files succeed or none are modified — backups created by default
toggle -S db:postgres --atomic -R src/

# Recover from an interrupted atomic run
toggle --recover            # rolls back
toggle --recover --recover-forward   # completes the commit
```

## Distribution

- **From source:** `cargo install --path .`
- **Shell completions:** `toggle --completions bash > /etc/bash_completion.d/toggle`
  (also `zsh`, `fish`, `powershell`, `elvish`)
- **Man page:** `toggle --man > toggle.1 && man ./toggle.1`
- **crates.io:** [`togl`](https://crates.io/crates/togl) — installs both `toggle` and `togl` binaries (same behavior under either name).
- **Homebrew:** not yet published.

---

## Reference

The remainder of this README is the original design spec, retained for
historical context. CLI semantics in the spec match what's implemented unless
called out above.

## 1. Overview

**Goal**  
Create a Rust-based CLI tool, **`toggle`**, that can:
- Comment or uncomment designated lines or blocks of text in code files.
- Detect and apply correct single-line or multi-line comment styles by file extension.
- Work off a configuration file (`.toggleConfig`) or command-line arguments.
- Identify labeled "sections" to toggle on or off across multiple files.
- Provide granular control (line-based, section-based, file-based, multi-file).

**Core Objectives**  
1. **Line-based toggling**: Support start/end line numbers, or a start line with a fixed number of lines, or a start line to the end of the file.  
2. **Section-based toggling**: Recognize in-file sections tagged with an ID (e.g., `SECTION_ID=foo`) and toggle all occurrences (on/off) across a codebase.  
3. **Configurable comment styles**: Auto-detect comment style by file extension or override with custom settings.  
4. **Extendable**: Allow a `.toggleConfig` file to hold global or per-language comment preferences.

---

## 2. Command-Line Interface (CLI)

### 2.1 Basic Command Syntax

```
toggle [OPTIONS] <file_or_directory_paths>...
```

### 2.2 Primary Flags & Arguments

| **Flag / Arg**               | **Description**                                                                                                     | **Example**                                    |
|------------------------------|---------------------------------------------------------------------------------------------------------------------|------------------------------------------------|
| `-l, --line` (repeatable)    | Specify line-based toggles in the format `<start_line>:<end_line>` or `<start_line>:+<count>`.                      | `--line 10:20` or `--line 15:+5`               |
| `-S, --section` (repeatable) | Specify section ID(s) to toggle.                                                                                   | `--section featureXYZ`                         |
| `-f, --force [on\|off]`      | Force a toggle state for line-based or section-based operations.                                                   | `--force on`                                   |
| `-m, --mode [auto\|single\|multi]` | Defines the comment mode. `auto` will use file extension to determine the style, `single`/`multi` overrides. | `--mode single`                                |
| `-c, --comment-style`        | Manually specify exact delimiters for single/multi-line comments (overrides auto detection).                       | `--comment-style "//" "/*" "*/"`              |
| `--to-end`                   | If set, toggling continues from `<start_line>` to the end of the file.                                             | `--line 50 --to-end`                           |
| `--config <path>`            | Points to a custom `.toggleConfig` file. Default is `.toggleConfig` in current directory if present.               | `--config /path/to/altConfig`                  |
| `-R, --recursive`            | Recursively search directories for files that match the toggled sections or line references.                       | `-R src/`                                      |
| `-v, --verbose`              | Show detailed logs (lines changed, files modified, etc.).                                                          | `--verbose`                                    |
| `--dry-run`                  | Show which changes **would** be made, without altering files.                                                       | `--dry-run --verbose`                          |

### 2.3 Behavior Examples

1. **Line Range Toggle**  
   ```bash
   toggle --line 10:20 main.py
   ```
   - Auto-detects `.py` → uses `#` for single-line comments.
   - Comments out lines 10 to 20 (or toggles them if already commented).

2. **Line Range to End**  
   ```bash
   toggle --line 30 --to-end MyClass.java
   ```
   - Auto-detects `.java` → uses `//` or `/*...*/`.
   - Comments out from line 30 to EOF.

3. **Section-Based Toggle**  
   ```bash
   toggle --section signupFlow --force on src/
   ```
   - Recursively scans `src/` to find any sections labeled `signupFlow`.
   - Forces them all to become commented (on).

4. **Override Comment Style**  
   ```bash
   toggle --mode multi --comment-style "//" "/*" "*/" --line 40:45 test.cc
   ```
   - Forces multi-line mode but uses custom single-line prefix `//` if needed.  
   - The multi-line delimiters are explicitly `/*` and `*/`.

5. **Multiple Toggles in One Command**  
   ```bash
   toggle --line 10:20 --section adminUI --force off module.ts
   ```
   - Toggles lines 10–20 and a named section `adminUI` in `module.ts`.
   - Forces off any commented region that is identified by `adminUI`.

---

## 3. Configuration File (`.toggleConfig`)

### 3.1 Purpose
- Defines default behavior per file extension or globally.
- Acts as the fallback if command-line arguments are not specified.

### 3.2 Format

```toml
[global]
default_mode = "auto"
force_state = "none"  # valid: on, off, none (i.e., invert if toggling)
single_line_delimiter = "//"  
multi_line_delimiter_start = "/*"
multi_line_delimiter_end = "*/"

[language.python]
single_line_delimiter = "#"
multi_line_delimiter_start = "\"\"\""
multi_line_delimiter_end = "\"\"\""

[language.ruby]
single_line_delimiter = "#"

[language.java]
single_line_delimiter = "//"
multi_line_delimiter_start = "/*"
multi_line_delimiter_end = "*/"
```

**Notes**:
- `global` section sets the baseline.
- Each `[language.xxx]` overrides settings for `.xxx` files.
- If no extension is recognized, the program either throws an error or uses `global` defaults.

---

## 4. Section Markers in Source Files

### 4.1 Marker Convention

A standard marker might look like this (example for Java/JS/C-style):

```java
// toggle:start ID=featureX desc="Enable the new feature"
  System.out.println("New feature code here...");
// toggle:end ID=featureX
```

Or for Python:

```python
# toggle:start ID=featureX desc="Enable the new feature"
print("New feature code here...")
# toggle:end ID=featureX
```

**Proposed Format**:  
```
[toggle:start ID=<identifier> desc="<description>"]
...
[toggle:end ID=<identifier>]
```

- **`ID`** is mandatory.  
- **`desc`** is optional.  
- The line format must be recognizable by `toggle`. For example:  
  - `// toggle:start ID=featureX desc="..."`  
  - `# toggle:start ID=featureX desc="..."`  
  - `/* toggle:start ID=featureX desc="..." */` (depending on language)

### 4.2 Behavior

- **Toggling ON**: If the block is not commented, comment it out. If already commented, do nothing.  
- **Toggling OFF**: If the block is commented, uncomment it. If already uncommented, do nothing.  
- **No Force**: If neither `--force on` nor `--force off` is set, `toggle` inverts the current state.  
- **Global Toggle**: The tool can scan multiple files (via `-R` or listing files) and apply toggles to all occurrences of an `ID`.

---

## 5. Implementation Outline (Rust)

1. **Argument Parsing**  
   - Use a crate like [**Clap**](https://docs.rs/clap/latest/clap/) or [**StructOpt**](https://docs.rs/structopt/) for robust CLI handling.
   - Collect line toggles (`Vec<String>` for `<start_line>:<end_line>`, etc.), sections, force states, and mode overrides.

2. **Configuration Handling**  
   - On startup, attempt to load `.toggleConfig` (or alternative path if `--config` is specified).
   - Parse with a TOML library (e.g., [**toml**](https://docs.rs/toml/latest/toml/)).
   - Merge config values with command-line overrides.

3. **File Scanner**  
   - If user inputs directories and `-R` is set, recursively walk the directory using [**walkdir**](https://docs.rs/walkdir/latest/walkdir/).
   - Filter files by extension or by presence of `toggle` markers.

4. **Comment Style Determination**  
   - If `--mode auto`, map extension → comment style. If not found, throw an error.  
   - If `--comment-style ...` is passed, override the style.  
   - If `.toggleConfig` contains `[language.xyz]` that matches extension, use those defaults unless overridden.

5. **Parsing the File**  
   - For each file, read line by line into a buffer (e.g., `Vec<String>`).
   - For line-based toggles:
     - Identify the relevant range(s).  
     - Comment or uncomment accordingly.
   - For section-based toggles:
     - Detect lines that match `toggle:start ID=...` and `toggle:end ID=...`.  
     - Determine if the block is currently commented or not.  
     - Apply on/off or invert logic.

6. **Commenting / Uncommenting Logic**  
   - **Single-line** approach (e.g., `#`, `//`): Prepend or remove the token from each line.  
   - **Multi-line** approach (e.g., `/* ... */`):
     - Insert `/*` at the first line, `*/` at the last line (or for partial lines, handle carefully).  
     - Alternatively, comment each line singly if that's simpler for toggling.  
   - Keep track of lines that are already partially or fully commented to avoid double-commenting.

7. **Output & Write-Back**  
   - After toggling, write the modified buffer back to the file (unless `--dry-run` is set).  
   - If `--dry-run`, print a summary of changes.

8. **Edge Cases**  
   - Overlapping toggles for the same lines or sections.  
   - Nested sections (some languages permit nested comment blocks).  
   - Files with unusual line endings (CRLF vs. LF).  
   - Extremely large files (consider streaming vs. loading entire file).

---

## 6. Example Use Case Scenarios

**Scenario A: Toggling a Feature in Multiple Files**  
```
toggle --section featureX --force off -R src/
```
- Recursively looks for `toggle:start ID=featureX`/`toggle:end ID=featureX`.
- Forces it off. If some blocks were on, they get uncommented.

**Scenario B: Automated Build Script**  
- Integrate `toggle` in a CI script to enable certain code blocks for a staging environment:
  ```bash
  toggle --section stagingFeature --force on path/to/config.yaml path/to/server.java
  ```
- Re-run with `--force off` after tests complete.

---

## 7. Error Handling & Logging

1. **Unknown Extension**  
   - If `--mode auto` and the file extension has no known mapping, error: “Cannot detect comment style for ‘.xyz’. Use `--mode` or `.toggleConfig` to specify.”

2. **Conflicting Options**  
   - If `--mode single` and `--comment-style` multi-line tokens are provided, prefer the explicit `--comment-style` or show a warning and proceed with single-line prepends.

3. **Invalid Ranges**  
   - If `start_line` > `end_line`, skip or warn.  
   - If lines exceed file length, skip out-of-bound lines and log a warning.

4. **Section Mismatch**  
   - If `toggle:start ID=foo` is found but no matching `toggle:end ID=foo`, log a warning: “Unclosed section ID=foo in filename.”

5. **Verbose Logging**  
   - If `-v, --verbose`, show each line range or section ID processed and the new state.
