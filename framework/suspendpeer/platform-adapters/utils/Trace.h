// Our tracing-only adapter. Binder calls and power.cpp logic execute unchanged.
#pragma once
#define ATRACE_TAG_POWER 0
#define ATRACE_CALL() ((void)0)
