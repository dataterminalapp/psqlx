/* Define PG_PRINTF_ATTRIBUTE */
#define PG_PRINTF_ATTRIBUTE gnu_printf

/* GCC supports format attributes */
#if defined(__GNUC__)
#define pg_attribute_format_arg(a) __attribute__((format_arg(a)))
#define pg_attribute_printf(f,a) __attribute__((format(PG_PRINTF_ATTRIBUTE, f, a)))
#else
#define pg_attribute_format_arg(a)
#define pg_attribute_printf(f,a)
#endif

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdarg.h>
#include <fe_utils/psqlscan.h>
#include <fe_utils/psqlscan_int.h>
#include "postgres.h"
#include "command.h"
#include "psqlscanslash.h"
#include "settings.h"