#include <stdio.h>

typedef struct _Unwind_Context Unwind_Context;

unsigned long _Unwind_GetGR (struct _Unwind_Context *a, int b) {
    return 0;
}

void _Unwind_SetGR (struct _Unwind_Context *a, int b, unsigned long c) {}

unsigned long _Unwind_GetIP (struct _Unwind_Context *a) {
    return 0;
}

void _Unwind_SetIP (struct _Unwind_Context *a, unsigned long b) {}
