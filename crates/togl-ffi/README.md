# togl-ffi — `libtogl`

An ABI-stable C library exposing the string-core and read-only introspection of
[`togl-lib`](../togl-lib) for toggling code comments across languages.

Produces `libtogl.a` (static), `libtogl.so`/`.dylib` (shared), and a committed C
header at [`include/togl.h`](include/togl.h).

## ABI conventions

- Every function returns an `int` status: `0` (`TOGL_OK`) on success, negative on
  error. Result data is delivered through out-pointers.
- Error codes (stable, never reused): `-1` null pointer, `-2` invalid UTF-8,
  `-3` internal panic, `-4` operation failed, `-5` invalid argument.
  `togl_error_message(code)` returns a static description.
- **Memory:** every `char*` the library returns via an out-pointer is owned by the
  library and must be released with `togl_string_free`. Never call C `free()` on
  it. `togl_version()` returns a static pointer that must NOT be freed.
- No panic crosses the boundary (each call is wrapped in `catch_unwind`).
- Complex results are returned as **JSON strings**; the only boundary struct is
  `ToglRange { size_t start; size_t end; }` (1-based inclusive line range).
- `togl_abi_version()` returns the integer ABI version for runtime negotiation.

## Parameter encodings

- `force_mode`: `0` = invert, `1` = force-comment (on), `2` = force-uncomment (off).
- `comment_marker`: a single-line comment marker; pass `NULL` to default to `"#"`.
- `pair_only`: `0` = all groups, non-zero = pair-validation mode.

## Example

```c
#include "togl.h"
#include <stdio.h>

int main(void) {
    ToglRange ranges[1] = { { 1, 1 } };   /* line 1, 1-based inclusive */
    char *out = NULL;
    if (togl_toggle_comments("a\nb\n", ranges, 1, /*force=*/1, &out) == 0) {
        printf("%s\n", out);              /* "# a\nb" */
        togl_string_free(out);
    }

    char *json = NULL;
    if (togl_discover_sections("# toggle:start ID=foo\nx\n# toggle:end ID=foo\n", &json) == 0) {
        printf("%s\n", json);             /* JSON array of sections */
        togl_string_free(json);
    }
    return 0;
}
```

Build & link (static):

```sh
cc example.c -I path/to/togl-ffi/include path/to/libtogl.a -o example   # +(-lpthread -ldl -lm on Linux)
```

Or via pkg-config once installed: `cc example.c $(pkg-config --cflags --libs togl) -o example`.
