// Frozen missing-interface target. Include ONLY the selected current public header.
#include "pokelinux_render_executor.h"
[[maybe_unused]] static auto const create_session_executor = &pr_executor_create_v2;
[[maybe_unused]] static auto const invalidate_session = &pr_executor_invalidate_session_v2;
[[maybe_unused]] static auto const admit_session_change = &pr_executor_try_session_change_v2;
[[maybe_unused]] static auto const copy_reconciliation = &pr_executor_reconciliation_v2;
