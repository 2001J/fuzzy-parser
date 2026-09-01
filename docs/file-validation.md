# File validation

This document describes the independently reviewed local #5 implementation.
Verification passed on macOS and Linux, including the Linux-only non-UTF-8
filename case. This does not establish hosted CI or publication.

`parser-formats::open_validated_file(path, &FileValidationOptions)` selects an
enabled extension and returns an already-open regular file, its selected
`FileFormat`, and checked `size_bytes`. `FileFormat` contains only `Txt`, `Csv`,
and `Xlsx`. Defaults enable all three, limit bytes to 1 MiB, and explicitly use
`EmptyFilePolicy::Accept`. Callers can restrict `enabled_formats`, change
`max_bytes`, or choose `EmptyFilePolicy::Reject`. An empty enabled-format list
rejects every extension. Matching is ASCII case-insensitive on the final
extension; missing, non-UTF-8, or unknown extensions are rejected.

`read_txt_with_options(path, TxtOptions { limits, empty_policy })` uses only
`Txt` eligibility and the existing `TextLimits` as its single byte-limit
authority (1 MiB total, 64 KiB per line by default). `read_txt(path)` and
`read_input(InputSource::TxtFile(path), limits)` use the same validation with
explicit empty acceptance. `read_txt_bytes`, pasted text and stdin do not gain
path validation or empty rejection in this slice. A filename supplied as byte
API metadata is not interpreted as a filesystem extension.

For example, a caller can keep the default line limit, allow at most 4 KiB,
and reject zero bytes (the caller supplies the actual input path):

```rust
use parser_formats::{EmptyFilePolicy, TextLimits, TxtOptions, read_txt_with_options};

let options = TxtOptions {
    limits: TextLimits { max_bytes: 4096, ..TextLimits::default() },
    empty_policy: EmptyFilePolicy::Reject,
};
let document = read_txt_with_options("input.txt", options);
```

Validation obtains path metadata, rejects nonregular inputs, validates the
extension and checks metadata size/empty policy before open, then opens for
reading and rechecks type/size/empty policy on that handle. Metadata checks do
not read the file contents. TXT consumes that same handle with the existing
bounded reader (at most limit plus one bytes), rejects actual zero bytes when
requested, then checks line limits and decodes UTF-8. Source size records actual
consumed bytes, not an earlier metadata snapshot. Exact-size inputs are allowed;
whitespace-only inputs are nonempty. OS metadata/open/read failures retain typed
I/O causes; mode bits are not a portable readability test.
If the OS rejects a pathname during metadata lookup (for example invalid UTF-8
filename bytes on macOS), its I/O error is returned before extension selection.
On filesystems that accept such names, non-UTF-8 extensions are rejected with
`unsupported_input`; neither path attempts content decoding.

## Compatibility and limits

The [error additions and compatibility changes](data-contracts.md#file-validation-additions-in-error-contract-01)
cover strict TXT extensions, directory errors, metadata oversize, and empty
rejection. The original #5 change rejected the CLI's TXT fallback through its
existing library call. The subsequent [CLI contract](integration-strategy.md#cli-grammar-and-validation-options)
now routes extensions explicitly and exposes TXT-only trailing byte/empty
overrides. It calls `read_txt_with_options` directly. CSV/XLSX path readers now
perform their own regular-file check and bounded read on the opened handle under
the [resource-limit contract](data-contracts.md#resource-limits--implemented);
TXT CLI overrides do not configure those limits.

Extensions are eligibility hints, not MIME verification or content sniffing.
CSV/XLSX eligibility in this helper does not integrate their current readers,
make zero-byte workbooks valid, or bound expanded workbook contents. Implemented
row, cell, schema and output limits have the documented post-materialization
boundaries and are not total process-memory guarantees. No malware scanning is
performed.

Symlinks are followed, including symlinks outside a caller's directory. The
supplied path's extension/name is used, and the target must be regular. The
precheck rejects ordinary special files before potentially blocking opens;
rechecking the opened handle avoids trusting stale path metadata. However,
another process can replace the path between precheck and open (even with a
blocking special file), and can modify/truncate/grow an opened file. Bounded
reads limit growth but do not provide a coherent snapshot. This is not a
filesystem sandbox, symlink-confinement guarantee or race-free opener. Callers
requiring stable or adversarial-filesystem isolation must stage inputs safely.
