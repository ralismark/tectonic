/* tectonic/core-bridge.h: declarations of C/C++ => Rust bridge API
   Copyright 2016-2020 the Tectonic Project
   Licensed under the MIT License.
*/

#ifndef TECTONIC_CORE_BRIDGE_H
#define TECTONIC_CORE_BRIDGE_H

#include "core-foundation.h"
#include "core-bindgen.h"

#include <time.h> /* time_t */

/* Both XeTeX and bibtex use this enum: */

typedef enum {
    HISTORY_SPOTLESS = 0,
    HISTORY_WARNING_ISSUED = 1,
    HISTORY_ERROR_ISSUED = 2,
    HISTORY_FATAL_ERROR = 3
} tt_history_t;

/* The weird enum values are historical and could be rationalized. But it is
 * good to write them explicitly since they must be kept in sync with
 * `src/engines/mod.rs`.
 */

typedef enum
{
    TTIF_TFM = 3,
    TTIF_AFM = 4,
    TTIF_BIB = 6,
    TTIF_BST = 7,
    TTIF_CNF = 8,
    TTIF_FORMAT = 10,
    TTIF_FONTMAP = 11,
    TTIF_OFM = 20,
    TTIF_OVF = 23,
    TTIF_PICT = 25,
    TTIF_TEX = 26,
    TTIF_TEX_PS_HEADER = 30,
    TTIF_TYPE1 = 32,
    TTIF_VF = 33,
    TTIF_TRUETYPE = 36,
    TTIF_BINARY = 40,
    TTIF_MISCFONTS = 41,
    TTIF_ENC = 44,
    TTIF_CMAP = 45,
    TTIF_SFD = 46,
    TTIF_OPENTYPE = 47,
    TTIF_TECTONIC_PRIMARY = 59, /* quasi-hack to get the primary input */
} tt_input_format_type;

typedef OutputHandle *rust_output_handle_t;
typedef InputHandle *rust_input_handle_t;
typedef Diagnostic *diagnostic_t;

BEGIN_EXTERN_C

/* The internal, C/C++ interface: */

NORETURN PRINTF_FUNC(1,2) int _tt_abort(const char *format, ...);

/* Global symbols that route through the global API variable. Hopefully we
 * will one day eliminate all of the global state and get rid of all of
 * these. */

// See xetex-xetexd.h for other useful functions for producing diagnostics.

// Finish and emit a diagnostic
void ttstub_diag_finish(diagnostic_t diag);
// Convenience functions to append to the message using printf specifiers
PRINTF_FUNC(2,3) void ttstub_diag_printf(diagnostic_t diag, const char *format, ...);
// Append to diagnostic message - for higher-level abstractions.
// Zero means there's no variadic parameter - only the format is checked. See
// https://gcc.gnu.org/onlinedocs/gcc/Common-Function-Attributes.html
PRINTF_FUNC(2,0) void ttstub_diag_vprintf(diagnostic_t diag, const char *format, va_list ap);

PRINTF_FUNC(1,2) void ttstub_issue_warning(const char *format, ...);
PRINTF_FUNC(1,2) void ttstub_issue_error(const char *format, ...);
PRINTF_FUNC(2,3) int ttstub_fprintf(rust_output_handle_t handle, const char *format, ...);

int ttstub_get_file_md5 (char const *path, char *digest);
int ttstub_get_data_md5 (char const *data, size_t len, char *digest);

rust_output_handle_t ttstub_output_open (char const *path, int is_gz);
rust_output_handle_t ttstub_output_open_stdout (void);
int ttstub_output_putc (rust_output_handle_t handle, int c);
size_t ttstub_output_write (rust_output_handle_t handle, const char *data, size_t len);
int ttstub_output_flush (rust_output_handle_t handle);
int ttstub_output_close (rust_output_handle_t handle);

rust_input_handle_t ttstub_input_open (char const *path, tt_input_format_type format, int is_gz);
rust_input_handle_t ttstub_input_open_primary (void);
size_t ttstub_input_get_size (rust_input_handle_t handle);
time_t ttstub_input_get_mtime (rust_input_handle_t handle);
size_t ttstub_input_seek (rust_input_handle_t handle, ssize_t offset, int whence);
ssize_t ttstub_input_read (rust_input_handle_t handle, char *data, size_t len);
int ttstub_input_getc (rust_input_handle_t handle);
int ttstub_input_ungetc (rust_input_handle_t handle, int ch);
int ttstub_input_close (rust_input_handle_t handle);

END_EXTERN_C

#endif /* not TECTONIC_CORE_BRIDGE_H */
