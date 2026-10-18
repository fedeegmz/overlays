# Delta Spec: path-template-variables (new capability)

## Purpose

OBS-style variable expansion for the upload destination template and the uploader-supplied file name, plus path-safety rules for names.

## Requirements

### Requirement: Supported variables
Expansion MUST use the machine's local time, captured once per expansion so all variables in one string are consistent. Supported variables:

| Variable | Meaning | Format |
|----------|---------|--------|
| `%CCYY` | year | 4 digits |
| `%YY` | year | 2 digits |
| `%MM` | month | 01-12 |
| `%DD` | day of month | 01-31 |
| `%hh` | hour | 00-23 |
| `%mm` | minute | 00-59 |
| `%ss` | second | 00-59 |
| `%%` | literal `%` | `%` |

Variables are case-sensitive (`%MM` is month, `%mm` is minute). Numeric values MUST be zero-padded.

#### Scenario: Full timestamp
- GIVEN local time 2026-03-07 09:05:02
- WHEN expanding `%CCYY-%MM-%DD_%hh-%mm-%ss`
- THEN the result is `2026-03-07_09-05-02`

#### Scenario: Two-digit year
- GIVEN local year 2026
- WHEN expanding `%YY`
- THEN the result is `26`

#### Scenario: Case sensitivity
- GIVEN local time 14:30
- WHEN expanding `%hh%mm` and `%HH%MM`
- THEN the first yields `1430`; `%HH`/`%MM` are not interpreted as hour/minute (`%MM` is month, `%HH` is unknown and left as-is)

#### Scenario: Escaped percent
- WHEN expanding `100%%`
- THEN the result is `100%`

#### Scenario: Escape prevents expansion
- WHEN expanding `%%CCYY`
- THEN the result is `%CCYY` (literal, not the year)

### Requirement: Unknown sequences left as-is
Any `%` sequence that is not a supported variable MUST be left unchanged in the output, including a trailing lone `%`. Locale-dependent variables (`%A`, `%B`, `%a`, `%b`, `%p`, etc.) are not supported and MUST be treated as unknown.

#### Scenario: Unknown variable
- WHEN expanding `x-%Q-y`
- THEN the result is `x-%Q-y`

#### Scenario: Trailing percent
- WHEN expanding `abc%`
- THEN the result is `abc%`

#### Scenario: Locale-dependent variable not supported
- WHEN expanding `%A`
- THEN the result is `%A`

#### Scenario: Longest-match precedence
- WHEN expanding `%CCYYx`
- THEN the result is the 4-digit year followed by `x`

### Requirement: Applies to destination template and name
The same expansion MUST be applied to the configured destination template and to the uploader's optional name field. The original uploaded file name (used when the name field is empty) MUST NOT be expanded; it is taken literally.

#### Scenario: Destination template
- GIVEN template `/srv/up/%CCYY/%MM` and local date 2026-03-07
- THEN uploads go to `/srv/up/2026/03`

#### Scenario: Name field
- GIVEN name `clip-%hh%mm` and local time 09:05
- THEN the stored name is `clip-0905`

#### Scenario: Original name not expanded
- GIVEN name field is empty and the uploaded file is called `100%%.txt`
- THEN the stored name is `100%%.txt`

### Requirement: Name sanitization
The file name (typed name or original name) MUST resolve to a single path component. Path separators (`/` and `\`) and `..` components MUST be rejected or stripped, and this MUST hold both before and after variable expansion, on all operating systems. NUL and other control characters MUST be removed. Leading/trailing whitespace SHOULD be trimmed. Characters invalid for the host filesystem (e.g. `:*?"<>|` on Windows) SHOULD be replaced or removed. If nothing remains, the caller MUST fall back as defined in `lan-file-upload` (original name, then `upload`).

#### Scenario: Traversal in name
- WHEN name is `../../evil.sh`
- THEN the stored file is inside the destination dir and its name contains no separators or `..` component (e.g. `evil.sh`)

#### Scenario: Backslash separators
- WHEN name is `a\b\c.txt`
- THEN the result is a single component with no separator (e.g. `c.txt` or `abc.txt`)

#### Scenario: Traversal produced by expansion
- GIVEN a name that only forms a separator or `..` after expansion (e.g. through `%%` combined with adjacent characters)
- THEN the post-expansion result is sanitized again and still cannot escape the destination

#### Scenario: Original filename with path
- WHEN the browser reports the original name as `C:\Users\x\photo.jpg` or `dir/photo.jpg`
- THEN only the final component `photo.jpg` is used

#### Scenario: Only dots
- WHEN name is `..` or `.`
- THEN the sanitized result is empty and the fallback name is used

#### Scenario: Control characters
- WHEN name contains a NUL or newline
- THEN those characters are removed

### Requirement: Destination template is trusted configuration
The destination template is user-configured by the machine owner and MAY contain path separators (it defines a directory path). Variables MUST be expanded in it, but values substituted by variables are digits only, so expansion cannot introduce traversal. A template that is relative or contains `..` components MUST be rejected at configuration time (see upload-settings).

#### Scenario: Multi-level directory template
- GIVEN template `/data/%CCYY/%MM/%DD`
- THEN directories are created as needed under `/data`

#### Scenario: Windows-style separators in template
- GIVEN template `D:\Uploads\%CCYY`
- THEN it is accepted on Windows and expands to `D:\Uploads\2026`
