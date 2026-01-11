#define VMA_IMPLEMENTATION

#ifdef _WIN32
#define VMA_CALL_PRE __declspec(dllexport)
#define VMA_CALL_POST __cdecl
#else
#define VMA_CALL_PRE
#define VMA_CALL_POST
#endif

#ifdef _WIN32
#define VK_USE_PLATFORM_WIN32_KHR
#endif

#include "vk_mem_alloc.h"