#pragma once

#include <cstdint>
#define VK_USE_PLATFORM_WAYLAND_KHR
#include <vulkan/vulkan.h>

#include <wayland-client.h>

// wayland-scanner may emit a parameter named `namespace`, which is a C++ keyword.
#define namespace namespace_renamed
#include "fractional-scale-v1-client-protocol.h"
#include "wlr-layer-shell-unstable-v1-client-protocol.h"
#undef namespace

#ifdef __cplusplus
extern "C" {
#endif

// Opaque window handle
typedef struct rwl_window rwl_window;

typedef enum rwl_window_type {
    RWL_WINDOW_TYPE_BACKGROUND = 0,  // Desktop background (bottom layer, covers full screen)
    RWL_WINDOW_TYPE_TASKBAR = 1,     // Taskbar/panel (top layer, typically anchored to top)
} rwl_window_type;

// Callback for pixel size changes (e.g., when compositor configures the surface)
typedef void (*rwl_pixel_size_callback)(rwl_window* window, uint32_t width, uint32_t height);
typedef void (*rwl_logical_size_callback)(rwl_window* window, uint32_t width, uint32_t height);

// Callback for frame timing (compositor is ready for next frame)
typedef void (*rwl_frame_callback)(rwl_window* window);

// Callback for logging messages from the library
// severity: 0=Debug, 1=Info, 2=Warning, 3=Error
typedef void (*rwl_log_callback)(uint32_t severity, const char* message);

typedef enum rwl_status {
    RWL_STATUS_OK = 0,
    RWL_STATUS_ALREADY_INITIALIZED = 1,
    RWL_STATUS_NOT_INITIALIZED = 2,
    RWL_STATUS_NO_DISPLAY = 3,
    RWL_STATUS_NO_REGISTRY = 4,
    RWL_STATUS_NO_COMPOSITOR = 5,
    RWL_STATUS_NO_LAYER_SHELL = 6,
    RWL_STATUS_INVALID_ARGUMENT = 7,
    RWL_STATUS_INTERNAL_ERROR = 8,
} rwl_status;

rwl_status rwlStartup();
void rwlSetLogCallback(rwl_log_callback callback);
rwl_status rwlShutdown();

// Window management
rwl_status rwlCreateWindow(rwl_window_type type, wl_output* output, uint32_t width, uint32_t height,
                           rwl_window** window_out);
rwl_status rwlDestroyWindow(rwl_window* window);
rwl_status rwlSetWindowUserPointer(rwl_window* window, void* pointer);
void* rwlGetWindowUserPointer(rwl_window* window);
rwl_status rwlSetPixelSizeCallback(rwl_window* window,
                                         rwl_pixel_size_callback callback);
rwl_status rwlSetLogicalSizeCallback(rwl_window* window,
                                            rwl_logical_size_callback callback);
rwl_status rwlSetFrameCallback(rwl_window* window, rwl_frame_callback callback);

// Get the buffer size to render at (accounts for fractional scaling).
rwl_status rwlGetPixelSize(rwl_window* window, uint32_t* width, uint32_t* height);

// Get the logical surface size (the size reported by the compositor).
rwl_status rwlGetLogicalSize(rwl_window* window, uint32_t* width, uint32_t* height);

// Create a Vulkan surface for the given window.
// VkInstance must be valid with VK_KHR_wayland_surface extension.
// The created VkSurfaceKHR is returned via surface_out.
rwl_status rwlCreateSurface(VkInstance instance, rwl_window* window, VkSurfaceKHR* surface_out);

// Event loop control
rwl_status rwlProcessEvents();
bool rwlWindowShouldClose(rwl_window* window);
rwl_status rwlWindowRequestClose(rwl_window* window);

#ifdef __cplusplus
}
#endif