#pragma once

#include <cstdint>
#define VK_USE_PLATFORM_WAYLAND_KHR
#include <vulkan/vulkan.h>

#include <wayland-client.h>

// wayland-scanner may emit a parameter named `namespace`, which is a C++ keyword.
#define namespace namespace_renamed
#include "fractional-scale-v1-client-protocol.h"
#include "wlr-layer-shell-unstable-v1-client-protocol.h"
#include "xdg-shell-client-protocol.h"
#undef namespace

#ifdef __cplusplus
extern "C" {
#endif

// Opaque window handle
typedef struct rwl_window rwl_window;

typedef enum rwl_window_type {
    RWL_WINDOW_TYPE_NORMAL = 0,
    RWL_WINDOW_TYPE_BACKGROUND = 1,  // Desktop background (bottom layer, covers full screen)
    RWL_WINDOW_TYPE_TASKBAR = 2,     // Taskbar/panel (top layer, typically anchored to top)
    RWL_WINDOW_TYPE_POPUP = 3,       // Popup window (top layer, typically transient)
} rwl_window_type;

typedef enum rwl_action {
    RWL_ACTION_RELEASE = 0,
    RWL_ACTION_PRESS = 1,
} rwl_action;

typedef enum rwl_mod {
    RWL_MOD_NONE = 0,
    RWL_MOD_SHIFT = 1 << 0,
    RWL_MOD_CTRL = 1 << 1,
    RWL_MOD_ALT = 1 << 2,
    RWL_MOD_SUPER = 1 << 3,
} rwl_mod;

typedef enum rwl_key {
    RWL_KEY_UNKNOWN = -1,
    RWL_KEY_SPACE = 32,
    RWL_KEY_APOSTROPHE = 39,
    RWL_KEY_COMMA = 44,
    RWL_KEY_MINUS = 45,
    RWL_KEY_PERIOD = 46,
    RWL_KEY_SLASH = 47,
    RWL_KEY_0 = 48,
    RWL_KEY_1 = 49,
    RWL_KEY_2 = 50,
    RWL_KEY_3 = 51,
    RWL_KEY_4 = 52,
    RWL_KEY_5 = 53,
    RWL_KEY_6 = 54,
    RWL_KEY_7 = 55,
    RWL_KEY_8 = 56,
    RWL_KEY_9 = 57,
    RWL_KEY_SEMICOLON = 59,
    RWL_KEY_EQUAL = 61,
    RWL_KEY_A = 65,
    RWL_KEY_B = 66,
    RWL_KEY_C = 67,
    RWL_KEY_D = 68,
    RWL_KEY_E = 69,
    RWL_KEY_F = 70,
    RWL_KEY_G = 71,
    RWL_KEY_H = 72,
    RWL_KEY_I = 73,
    RWL_KEY_J = 74,
    RWL_KEY_K = 75,
    RWL_KEY_L = 76,
    RWL_KEY_M = 77,
    RWL_KEY_N = 78,
    RWL_KEY_O = 79,
    RWL_KEY_P = 80,
    RWL_KEY_Q = 81,
    RWL_KEY_R = 82,
    RWL_KEY_S = 83,
    RWL_KEY_T = 84,
    RWL_KEY_U = 85,
    RWL_KEY_V = 86,
    RWL_KEY_W = 87,
    RWL_KEY_X = 88,
    RWL_KEY_Y = 89,
    RWL_KEY_Z = 90,
    RWL_KEY_LEFT_BRACKET = 91,
    RWL_KEY_BACKSLASH = 92,
    RWL_KEY_RIGHT_BRACKET = 93,
    RWL_KEY_GRAVE_ACCENT = 96,
    RWL_KEY_WORLD_1 = 161,
    RWL_KEY_WORLD_2 = 162,
    RWL_KEY_ESCAPE = 256,
    RWL_KEY_ENTER = 257,
    RWL_KEY_TAB = 258,
    RWL_KEY_BACKSPACE = 259,
    RWL_KEY_INSERT = 260,
    RWL_KEY_DELETE = 261,
    RWL_KEY_RIGHT = 262,
    RWL_KEY_LEFT = 263,
    RWL_KEY_DOWN = 264,
    RWL_KEY_UP = 265,
    RWL_KEY_PAGE_UP = 266,
    RWL_KEY_PAGE_DOWN = 267,
    RWL_KEY_HOME = 268,
    RWL_KEY_END = 269,
    RWL_KEY_CAPS_LOCK = 280,
    RWL_KEY_SCROLL_LOCK = 281,
    RWL_KEY_NUM_LOCK = 282,
    RWL_KEY_PRINT_SCREEN = 283,
    RWL_KEY_PAUSE = 284,
    RWL_KEY_F1 = 290,
    RWL_KEY_F2 = 291,
    RWL_KEY_F3 = 292,
    RWL_KEY_F4 = 293,
    RWL_KEY_F5 = 294,
    RWL_KEY_F6 = 295,
    RWL_KEY_F7 = 296,
    RWL_KEY_F8 = 297,
    RWL_KEY_F9 = 298,
    RWL_KEY_F10 = 299,
    RWL_KEY_F11 = 300,
    RWL_KEY_F12 = 301,
    RWL_KEY_F13 = 302,
    RWL_KEY_F14 = 303,
    RWL_KEY_F15 = 304,
    RWL_KEY_F16 = 305,
    RWL_KEY_F17 = 306,
    RWL_KEY_F18 = 307,
    RWL_KEY_F19 = 308,
    RWL_KEY_F20 = 309,
    RWL_KEY_F21 = 310,
    RWL_KEY_F22 = 311,
    RWL_KEY_F23 = 312,
    RWL_KEY_F24 = 313,
    RWL_KEY_F25 = 314,
    RWL_KEY_KP_0 = 320,
    RWL_KEY_KP_1 = 321,
    RWL_KEY_KP_2 = 322,
    RWL_KEY_KP_3 = 323,
    RWL_KEY_KP_4 = 324,
    RWL_KEY_KP_5 = 325,
    RWL_KEY_KP_6 = 326,
    RWL_KEY_KP_7 = 327,
    RWL_KEY_KP_8 = 328,
    RWL_KEY_KP_9 = 329,
    RWL_KEY_KP_DECIMAL = 330,
    RWL_KEY_KP_DIVIDE = 331,
    RWL_KEY_KP_MULTIPLY = 332,
    RWL_KEY_KP_SUBTRACT = 333,
    RWL_KEY_KP_ADD = 334,
    RWL_KEY_KP_ENTER = 335,
    RWL_KEY_KP_EQUAL = 336,
    RWL_KEY_LEFT_SHIFT = 340,
    RWL_KEY_LEFT_CONTROL = 341,
    RWL_KEY_LEFT_ALT = 342,
    RWL_KEY_LEFT_SUPER = 343,
    RWL_KEY_RIGHT_SHIFT = 344,
    RWL_KEY_RIGHT_CONTROL = 345,
    RWL_KEY_RIGHT_ALT = 346,
    RWL_KEY_RIGHT_SUPER = 347,
    RWL_KEY_MENU = 348,
    RWL_KEY_COUNT = 349
} rwl_key;

// Callback for pixel size changes (e.g., when compositor configures the surface)
typedef void (*rwl_pixel_size_callback)(rwl_window* window, uint32_t width, uint32_t height);
// Callback for logical size changes (e.g., when compositor configures the surface)
typedef void (*rwl_logical_size_callback)(rwl_window* window, uint32_t width, uint32_t height);

// Callback for key events
typedef void (*rwl_key_callback)(
    rwl_window* window, int32_t key, int32_t scancode, int32_t action, int32_t mods);

// Callback for character input events (UTF-32 codepoint)
// Only triggered on key press, not release
typedef void (*rwl_char_callback)(rwl_window* window, uint32_t codepoint);

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

// Log severity levels matching the Rust log module
typedef enum rwl_log_severity {
    RWL_LOG_DEBUG = 0,
    RWL_LOG_INFO = 1,
    RWL_LOG_WARNING = 2,
    RWL_LOG_ERROR = 3,
} rwl_log_severity;

void rwlSetLogCallback(rwl_log_severity min_severity, rwl_log_callback callback);
rwl_status rwlStartup();
rwl_status rwlShutdown();

// Window management
rwl_status rwlCreateWindow(rwl_window_type type,
    wl_output* output,
    uint32_t width,
    uint32_t height,
    rwl_window** window_out);
rwl_status rwlDestroyWindow(rwl_window* window);

rwl_status rwlSetWindowUserPointer(rwl_window* window, void* pointer);
void* rwlGetWindowUserPointer(rwl_window* window);

rwl_status rwlSetPixelSizeCallback(rwl_window* window, rwl_pixel_size_callback callback);
rwl_status rwlSetLogicalSizeCallback(rwl_window* window, rwl_logical_size_callback callback);
rwl_status rwlSetKeyCallback(rwl_window* window, rwl_key_callback callback);
rwl_status rwlSetCharCallback(rwl_window* window, rwl_char_callback callback);

// Get the buffer size to render at (accounts for fractional scaling).
rwl_status rwlGetPixelSize(rwl_window* window, uint32_t* width, uint32_t* height);

// Get the logical surface size (the size reported by the compositor).
rwl_status rwlGetLogicalSize(rwl_window* window, uint32_t* width, uint32_t* height);

// Get the Wayland display and surface handles for the given window.
rwl_status rwlGetWaylandHandles(
    rwl_window* window, wl_display** out_display, wl_surface** out_surface);

// Information about a particular output
typedef struct rwl_output_info_public {
    // Opaque pointer to the wl_output
    void* wl_output;
    // Output name (e.g., "HDMI-1", "DP-2")
    const char* name;
    // Output description
    const char* description;
    // Scale factor
    int32_t scale;
    // Physical width in pixels
    int32_t width;
    // Physical height in pixels
    int32_t height;
} rwl_output_info_public;

// Get the list of available outputs
rwl_status rwlEnumerateOutputs(uint32_t* count, rwl_output_info_public* outputs_out);

// Event loop control
rwl_status rwlPollEvents();
rwl_status rwlWaitEvents();
rwl_status rwlWaitEventsTimeout(uint64_t timeout_ns);
rwl_status rwlPostEmptyEvent();
bool rwlWindowShouldClose(rwl_window* window);
rwl_status rwlWindowRequestClose(rwl_window* window);

#ifdef __cplusplus
}
#endif