#include "togl.h"
#include <assert.h>
#include <string.h>
#include <stdio.h>

int main(void) {
    /* metadata */
    assert(togl_abi_version() == 1);
    assert(strlen(togl_version()) > 0);
    assert(strcmp(togl_error_message(0), "ok") == 0);

    /* transform: comment line 1 (force_mode 1 = on) */
    ToglRange ranges[1] = { { 1, 1 } };
    char *out = NULL;
    int rc = togl_toggle_comments("a\nb\n", ranges, 1, 1, &out);
    assert(rc == 0);
    assert(strstr(out, "# a") != NULL);
    togl_string_free(out);

    /* introspection: JSON array */
    char *json = NULL;
    rc = togl_discover_sections("# toggle:start ID=foo\nx\n# toggle:end ID=foo\n", &json);
    assert(rc == 0);
    assert(json[0] == '[');
    togl_string_free(json);

    /* error path: NULL content → TOGL_ERR_NULL_POINTER (-1) */
    char *bad = NULL;
    assert(togl_toggle_comments(NULL, NULL, 0, 0, &bad) == -1);

    printf("C smoke test passed\n");
    return 0;
}
