# TXT regression fixtures

All values are synthetic. The byte-sensitive cases use whitespace-separated
hex pairs so editors and Git cannot trim significant spaces, convert line
endings, or replace invalid UTF-8. The
[TXT fixture tests](../../crates/parser-formats/tests/unit/txt_fixtures.rs) decode
each `.hex` file into an owned temporary `.txt` file and exercise `read_txt`
and, for successful reads, `read_input(InputSource::TxtFile)`.
No fixture is normalized or parsed for business meaning.

| Fixture | Exact content / purpose |
| --- | --- |
| [simple.txt](simple.txt) | Existing ordinary two-line UTF-8 fixture; retained `reads_repository_fixture` coverage |
| [unicode-whitespace.txt.hex](unicode-whitespace.txt.hex) | 65 bytes: Unicode names, em dash, curly quotes, CJK, emoji, tabs, leading/trailing spaces, a nonbreaking space, decomposed accent, and a synthetic phone-shaped string; final line has no terminator |
| [empty.txt](empty.txt) | Zero bytes; metadata remains present and there are no blocks |
| [blank-lines-lf.txt.hex](blank-lines-lf.txt.hex) | 17 bytes: `\n\nAlpha\n\n\nOmega\n\n`; seven blocks, including leading/interior/trailing empty lines |
| [blank-lines-crlf.txt.hex](blank-lines-crlf.txt.hex) | 24 bytes: the same seven lines with `\r\n` terminators and different byte offsets |
| [invalid-utf8.txt.hex](invalid-utf8.txt.hex) | Seven bytes: `ok\n`, UTF-8 `é`, invalid byte `ff`, then LF; decoding fails at byte offset 5 without replacement text |

New successful fixture cases assert the complete raw document, including source type,
basename, MIME type, byte size, absent delimiter, ordered block IDs, one-based
lines, exact original byte slices, and no warnings. Repeated path reads and the
dispatcher must return identical documents. The unchanged
[raw-document contract](../../docs/data-contracts.md#canonical-raw-document)
defines blank lines and the absence of a phantom block after a final terminator.

Failure tests create a missing child path in a fresh temporary directory and
pass the directory itself as an unreadable file. Missing paths retain typed I/O
causes; #5 deliberately changes directories to `not_regular_file` before open,
preserving the original regression name. A synthetic reader returns permission denied
after a partial read through the existing bounded TXT reader. This permanently
tests cause propagation without chmod, host paths, root-dependent skips, or
production test seams; it does not claim to test OS permission enforcement.
Error JSON/Display and redaction are owned by issue #2, not these fixtures.
