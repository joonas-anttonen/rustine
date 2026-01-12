#include "rustine-wl.hpp"

#include <linux/input.h>
#include <poll.h>
#include <sys/eventfd.h>
#include <sys/mman.h>
#include <unistd.h>
#include <xkbcommon/xkbcommon-compose.h>
#include <xkbcommon/xkbcommon.h>

#include <cerrno>
#include <cstring>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

static wl_display* g_display = nullptr;
static wl_registry* g_registry = nullptr;
static wl_compositor* g_compositor = nullptr;
static wl_seat* g_seat = nullptr;
static wl_keyboard* g_keyboard = nullptr;

static zwlr_layer_shell_v1* g_layer_shell = nullptr;
static wp_fractional_scale_manager_v1* g_fractional_scale_manager = nullptr;

// XDG
static xdg_wm_base* g_xdg_wm_base = nullptr;

// XKB
static xkb_context* g_xkb_context = nullptr;
static xkb_keymap* g_xkb_keymap = nullptr;
static xkb_state* g_xkb_state = nullptr;
static xkb_compose_state* g_xkb_compose_state = nullptr;
static xkb_mod_index_t g_xkb_mod_shift;
static xkb_mod_index_t g_xkb_mod_ctrl;
static xkb_mod_index_t g_xkb_mod_alt;
static xkb_mod_index_t g_xkb_mod_super;

static rwl_mod g_mods = RWL_MOD_NONE;
static rwl_key g_keycodes[256];
static rwl_key g_scancodes[RWL_KEY_COUNT + 1];
static void create_key_tables() {
    memset(g_keycodes, -1, sizeof(g_keycodes));
    memset(g_scancodes, -1, sizeof(g_scancodes));

    g_keycodes[KEY_GRAVE] = RWL_KEY_GRAVE_ACCENT;
    g_keycodes[KEY_1] = RWL_KEY_1;
    g_keycodes[KEY_2] = RWL_KEY_2;
    g_keycodes[KEY_3] = RWL_KEY_3;
    g_keycodes[KEY_4] = RWL_KEY_4;
    g_keycodes[KEY_5] = RWL_KEY_5;
    g_keycodes[KEY_6] = RWL_KEY_6;
    g_keycodes[KEY_7] = RWL_KEY_7;
    g_keycodes[KEY_8] = RWL_KEY_8;
    g_keycodes[KEY_9] = RWL_KEY_9;
    g_keycodes[KEY_0] = RWL_KEY_0;
    g_keycodes[KEY_SPACE] = RWL_KEY_SPACE;
    g_keycodes[KEY_MINUS] = RWL_KEY_MINUS;
    g_keycodes[KEY_EQUAL] = RWL_KEY_EQUAL;
    g_keycodes[KEY_Q] = RWL_KEY_Q;
    g_keycodes[KEY_W] = RWL_KEY_W;
    g_keycodes[KEY_E] = RWL_KEY_E;
    g_keycodes[KEY_R] = RWL_KEY_R;
    g_keycodes[KEY_T] = RWL_KEY_T;
    g_keycodes[KEY_Y] = RWL_KEY_Y;
    g_keycodes[KEY_U] = RWL_KEY_U;
    g_keycodes[KEY_I] = RWL_KEY_I;
    g_keycodes[KEY_O] = RWL_KEY_O;
    g_keycodes[KEY_P] = RWL_KEY_P;
    g_keycodes[KEY_LEFTBRACE] = RWL_KEY_LEFT_BRACKET;
    g_keycodes[KEY_RIGHTBRACE] = RWL_KEY_RIGHT_BRACKET;
    g_keycodes[KEY_A] = RWL_KEY_A;
    g_keycodes[KEY_S] = RWL_KEY_S;
    g_keycodes[KEY_D] = RWL_KEY_D;
    g_keycodes[KEY_F] = RWL_KEY_F;
    g_keycodes[KEY_G] = RWL_KEY_G;
    g_keycodes[KEY_H] = RWL_KEY_H;
    g_keycodes[KEY_J] = RWL_KEY_J;
    g_keycodes[KEY_K] = RWL_KEY_K;
    g_keycodes[KEY_L] = RWL_KEY_L;
    g_keycodes[KEY_SEMICOLON] = RWL_KEY_SEMICOLON;
    g_keycodes[KEY_APOSTROPHE] = RWL_KEY_APOSTROPHE;
    g_keycodes[KEY_Z] = RWL_KEY_Z;
    g_keycodes[KEY_X] = RWL_KEY_X;
    g_keycodes[KEY_C] = RWL_KEY_C;
    g_keycodes[KEY_V] = RWL_KEY_V;
    g_keycodes[KEY_B] = RWL_KEY_B;
    g_keycodes[KEY_N] = RWL_KEY_N;
    g_keycodes[KEY_M] = RWL_KEY_M;
    g_keycodes[KEY_COMMA] = RWL_KEY_COMMA;
    g_keycodes[KEY_DOT] = RWL_KEY_PERIOD;
    g_keycodes[KEY_SLASH] = RWL_KEY_SLASH;
    g_keycodes[KEY_BACKSLASH] = RWL_KEY_BACKSLASH;
    g_keycodes[KEY_ESC] = RWL_KEY_ESCAPE;
    g_keycodes[KEY_TAB] = RWL_KEY_TAB;
    g_keycodes[KEY_LEFTSHIFT] = RWL_KEY_LEFT_SHIFT;
    g_keycodes[KEY_RIGHTSHIFT] = RWL_KEY_RIGHT_SHIFT;
    g_keycodes[KEY_LEFTCTRL] = RWL_KEY_LEFT_CONTROL;
    g_keycodes[KEY_RIGHTCTRL] = RWL_KEY_RIGHT_CONTROL;
    g_keycodes[KEY_LEFTALT] = RWL_KEY_LEFT_ALT;
    g_keycodes[KEY_RIGHTALT] = RWL_KEY_RIGHT_ALT;
    g_keycodes[KEY_LEFTMETA] = RWL_KEY_LEFT_SUPER;
    g_keycodes[KEY_RIGHTMETA] = RWL_KEY_RIGHT_SUPER;
    g_keycodes[KEY_COMPOSE] = RWL_KEY_MENU;
    g_keycodes[KEY_NUMLOCK] = RWL_KEY_NUM_LOCK;
    g_keycodes[KEY_CAPSLOCK] = RWL_KEY_CAPS_LOCK;
    g_keycodes[KEY_PRINT] = RWL_KEY_PRINT_SCREEN;
    g_keycodes[KEY_SCROLLLOCK] = RWL_KEY_SCROLL_LOCK;
    g_keycodes[KEY_PAUSE] = RWL_KEY_PAUSE;
    g_keycodes[KEY_DELETE] = RWL_KEY_DELETE;
    g_keycodes[KEY_BACKSPACE] = RWL_KEY_BACKSPACE;
    g_keycodes[KEY_ENTER] = RWL_KEY_ENTER;
    g_keycodes[KEY_HOME] = RWL_KEY_HOME;
    g_keycodes[KEY_END] = RWL_KEY_END;
    g_keycodes[KEY_PAGEUP] = RWL_KEY_PAGE_UP;
    g_keycodes[KEY_PAGEDOWN] = RWL_KEY_PAGE_DOWN;
    g_keycodes[KEY_INSERT] = RWL_KEY_INSERT;
    g_keycodes[KEY_LEFT] = RWL_KEY_LEFT;
    g_keycodes[KEY_RIGHT] = RWL_KEY_RIGHT;
    g_keycodes[KEY_DOWN] = RWL_KEY_DOWN;
    g_keycodes[KEY_UP] = RWL_KEY_UP;
    g_keycodes[KEY_F1] = RWL_KEY_F1;
    g_keycodes[KEY_F2] = RWL_KEY_F2;
    g_keycodes[KEY_F3] = RWL_KEY_F3;
    g_keycodes[KEY_F4] = RWL_KEY_F4;
    g_keycodes[KEY_F5] = RWL_KEY_F5;
    g_keycodes[KEY_F6] = RWL_KEY_F6;
    g_keycodes[KEY_F7] = RWL_KEY_F7;
    g_keycodes[KEY_F8] = RWL_KEY_F8;
    g_keycodes[KEY_F9] = RWL_KEY_F9;
    g_keycodes[KEY_F10] = RWL_KEY_F10;
    g_keycodes[KEY_F11] = RWL_KEY_F11;
    g_keycodes[KEY_F12] = RWL_KEY_F12;
    g_keycodes[KEY_F13] = RWL_KEY_F13;
    g_keycodes[KEY_F14] = RWL_KEY_F14;
    g_keycodes[KEY_F15] = RWL_KEY_F15;
    g_keycodes[KEY_F16] = RWL_KEY_F16;
    g_keycodes[KEY_F17] = RWL_KEY_F17;
    g_keycodes[KEY_F18] = RWL_KEY_F18;
    g_keycodes[KEY_F19] = RWL_KEY_F19;
    g_keycodes[KEY_F20] = RWL_KEY_F20;
    g_keycodes[KEY_F21] = RWL_KEY_F21;
    g_keycodes[KEY_F22] = RWL_KEY_F22;
    g_keycodes[KEY_F23] = RWL_KEY_F23;
    g_keycodes[KEY_F24] = RWL_KEY_F24;
    g_keycodes[KEY_KPSLASH] = RWL_KEY_KP_DIVIDE;
    g_keycodes[KEY_KPASTERISK] = RWL_KEY_KP_MULTIPLY;
    g_keycodes[KEY_KPMINUS] = RWL_KEY_KP_SUBTRACT;
    g_keycodes[KEY_KPPLUS] = RWL_KEY_KP_ADD;
    g_keycodes[KEY_KP0] = RWL_KEY_KP_0;
    g_keycodes[KEY_KP1] = RWL_KEY_KP_1;
    g_keycodes[KEY_KP2] = RWL_KEY_KP_2;
    g_keycodes[KEY_KP3] = RWL_KEY_KP_3;
    g_keycodes[KEY_KP4] = RWL_KEY_KP_4;
    g_keycodes[KEY_KP5] = RWL_KEY_KP_5;
    g_keycodes[KEY_KP6] = RWL_KEY_KP_6;
    g_keycodes[KEY_KP7] = RWL_KEY_KP_7;
    g_keycodes[KEY_KP8] = RWL_KEY_KP_8;
    g_keycodes[KEY_KP9] = RWL_KEY_KP_9;
    g_keycodes[KEY_KPDOT] = RWL_KEY_KP_DECIMAL;
    g_keycodes[KEY_KPEQUAL] = RWL_KEY_KP_EQUAL;
    g_keycodes[KEY_KPENTER] = RWL_KEY_KP_ENTER;
    g_keycodes[KEY_102ND] = RWL_KEY_WORLD_2;

    for (int scancode = 0; scancode < 256; scancode++) {
        if (g_keycodes[scancode] > 0)
            g_scancodes[g_keycodes[scancode]] = static_cast<rwl_key>(scancode);
    }
}
static rwl_key translate_key(uint32_t scancode) {
    if (scancode < sizeof(g_keycodes) / sizeof(g_keycodes[0]))
        return g_keycodes[scancode];

    return RWL_KEY_UNKNOWN;
}

static rwl_log_callback g_log_callback = nullptr;
static rwl_log_severity g_log_severity = RWL_LOG_DEBUG;

static void rwl_log(rwl_log_severity severity, const char* message) {
    if (severity < g_log_severity) {
        return;
    }
    if (g_log_callback) {
        g_log_callback(static_cast<uint32_t>(severity), message);
    }
}

static int g_wake_fd = -1;
static std::vector<std::unique_ptr<struct rwl_window_internal>> g_windows;
static rwl_window_internal* g_window_with_keyboard = nullptr;

// Output information structure
struct rwl_output_info {
    uint32_t wl_name;
    wl_output* output;
    std::string name;
    std::string description;
    int32_t scale;
    int32_t width;
    int32_t height;
};

static std::vector<std::unique_ptr<rwl_output_info>> g_outputs;

// Internal window structure
struct rwl_window_internal {
    wl_surface* surface;
    wl_output* output;
    zwlr_layer_surface_v1* layer_surface;
    wp_fractional_scale_v1* fractional_scale;

    // XDG
    xdg_surface* xdg_surface;
    xdg_toplevel* xdg_toplevel;

    uint32_t width;
    uint32_t height;
    uint32_t preferred_fractional_scale;

    rwl_pixel_size_callback pixel_size_callback;
    rwl_logical_size_callback logical_size_callback;
    rwl_key_callback key_callback;
    rwl_char_callback char_callback;

    void* user_pointer;

    bool should_close;
    bool key_states[RWL_KEY_COUNT + 1];
};

static void rwl_cleanup_window(rwl_window_internal* window) {
    if (!window) {
        return;
    }

    if (g_window_with_keyboard == window) {
        g_window_with_keyboard = nullptr;
    }

    if (window->xdg_toplevel) {
        xdg_toplevel_destroy(window->xdg_toplevel);
        window->xdg_toplevel = nullptr;
    }

    if (window->xdg_surface) {
        xdg_surface_destroy(window->xdg_surface);
        window->xdg_surface = nullptr;
    }

    if (window->fractional_scale) {
        wp_fractional_scale_v1_destroy(window->fractional_scale);
        window->fractional_scale = nullptr;
    }

    if (window->layer_surface) {
        zwlr_layer_surface_v1_destroy(window->layer_surface);
        window->layer_surface = nullptr;
    }

    if (window->surface) {
        wl_surface_destroy(window->surface);
        window->surface = nullptr;
    }
}

static void rwl_request_close_all_windows() {
    for (auto& window : g_windows) {
        if (window) {
            window->should_close = true;
        }
    }
}

#ifdef __cplusplus
extern "C" {
#endif

// Layer surface listener callback
static void layer_surface_configure(void* data,
    struct zwlr_layer_surface_v1* surface,
    uint32_t serial,
    uint32_t width,
    uint32_t height) {
    rwl_window_internal* window = static_cast<rwl_window_internal*>(data);
    char buffer[256];
    snprintf(buffer,
        sizeof(buffer),
        "layer_surface_configure: width=%u height=%u serial=%u",
        width,
        height,
        serial);
    rwl_log(RWL_LOG_DEBUG, buffer);

    // Store logical size from compositor
    // This is the surface size in logical coordinates
    // The actual buffer size will be calculated using fractional scale in
    // rwlGetFramebufferSize()
    window->width = width;
    window->height = height;
    zwlr_layer_surface_v1_ack_configure(surface, serial);

    // Call logical size callback
    if (window->logical_size_callback) {
        window->logical_size_callback(reinterpret_cast<rwl_window*>(window), width, height);
    }

    // Calculate and call pixel size callback
    if (window->pixel_size_callback) {
        uint32_t pixel_width, pixel_height;
        if (window->preferred_fractional_scale != 120) {
            pixel_width = (width * window->preferred_fractional_scale + 60) / 120;
            pixel_height = (height * window->preferred_fractional_scale + 60) / 120;
        } else {
            pixel_width = width;
            pixel_height = height;
        }
        window->pixel_size_callback(
            reinterpret_cast<rwl_window*>(window), pixel_width, pixel_height);
    }
}

static void layer_surface_closed(void* data, struct zwlr_layer_surface_v1* surface) {
    // Compositor closed the layer surface; set window-specific close flag
    rwl_window_internal* window = static_cast<rwl_window_internal*>(data);
    rwl_log(RWL_LOG_INFO, "layer_surface_closed: window closed by compositor");
    window->should_close = true;
}

static const struct zwlr_layer_surface_v1_listener layer_surface_listener = {
    layer_surface_configure,
    layer_surface_closed,
};

// Fractional scale listener callback
static void window_fractional_scale_preferred_scale(
    void* data, struct wp_fractional_scale_v1* fractional_scale, uint32_t scale) {
    rwl_window_internal* window = static_cast<rwl_window_internal*>(data);
    window->preferred_fractional_scale = scale;

    char buffer[256];
    snprintf(buffer,
        sizeof(buffer),
        "window_fractional_scale_preferred_scale: scale=%u (%.2fx)",
        scale,
        scale / 120.0);
    rwl_log(RWL_LOG_INFO, buffer);

    // When fractional scale changes, recalculate and notify pixel size callback
    if (window->pixel_size_callback) {
        uint32_t pixel_width, pixel_height;
        if (scale != 120) {
            pixel_width = (window->width * scale + 60) / 120;
            pixel_height = (window->height * scale + 60) / 120;
        } else {
            pixel_width = window->width;
            pixel_height = window->height;
        }
        window->pixel_size_callback(
            reinterpret_cast<rwl_window*>(window), pixel_width, pixel_height);
    }
}

static const struct wp_fractional_scale_v1_listener window_fractional_scale_listener = {
    window_fractional_scale_preferred_scale,
};

// Output listener callbacks
static void output_geometry(void* data,
    struct wl_output* output,
    int32_t x,
    int32_t y,
    int32_t width_mm,
    int32_t height_mm,
    int32_t subpixel,
    const char* make,
    const char* model,
    int32_t transform) {
    char buffer[512];
    snprintf(buffer,
        sizeof(buffer),
        "output_geometry: make=%s model=%s x=%d y=%d width_mm=%d "
        "height_mm=%d subpixel=%d "
        "transform=%d",
        make,
        model,
        x,
        y,
        width_mm,
        height_mm,
        subpixel,
        transform);
    rwl_log(RWL_LOG_DEBUG, buffer);
}

static void output_mode(void* data,
    struct wl_output* output,
    uint32_t flags,
    int32_t width,
    int32_t height,
    int32_t refresh) {
    rwl_output_info* info = static_cast<rwl_output_info*>(data);
    info->width = width;
    info->height = height;

    char buffer[256];
    snprintf(buffer,
        sizeof(buffer),
        "output_mode: width=%d height=%d refresh=%d flags=0x%x",
        width,
        height,
        refresh,
        flags);
    rwl_log(RWL_LOG_DEBUG, buffer);
}

static void output_done(void* data, struct wl_output* output) {
    rwl_output_info* info = static_cast<rwl_output_info*>(data);
    char buffer[512];
    snprintf(buffer,
        sizeof(buffer),
        "output_done: name=%s description=%s width=%d height=%d scale=%d",
        info->name.empty() ? "(unknown)" : info->name.c_str(),
        info->description.empty() ? "(unknown)" : info->description.c_str(),
        info->width,
        info->height,
        info->scale);
    rwl_log(RWL_LOG_INFO, buffer);
}

static void output_scale(void* data, struct wl_output* output, int32_t scale) {
    rwl_output_info* info = static_cast<rwl_output_info*>(data);
    info->scale = scale;

    char buffer[256];
    snprintf(buffer, sizeof(buffer), "output_scale: scale=%d", scale);
    rwl_log(RWL_LOG_DEBUG, buffer);
}

static void output_name(void* data, struct wl_output* output, const char* name) {
    rwl_output_info* info = static_cast<rwl_output_info*>(data);
    info->name = name;

    char buffer[256];
    snprintf(buffer, sizeof(buffer), "output_name: %s", name);
    rwl_log(RWL_LOG_DEBUG, buffer);
}

static void output_description(void* data, struct wl_output* output, const char* description) {
    rwl_output_info* info = static_cast<rwl_output_info*>(data);
    info->description = description;

    char buffer[512];
    snprintf(buffer, sizeof(buffer), "output_description: %s", description);
    rwl_log(RWL_LOG_DEBUG, buffer);
}

static const struct wl_output_listener output_listener = {
    output_geometry,
    output_mode,
    output_done,
    output_scale,
    output_name,
    output_description,
};

static void wm_base_handle_ping(void* data, struct xdg_wm_base* wm_base, uint32_t serial) {
    xdg_wm_base_pong(wm_base, serial);
}

static const struct xdg_wm_base_listener wm_base_listener = {wm_base_handle_ping};

static void keyboard_handle_keymap(
    void* data, struct wl_keyboard* keyboard, uint32_t format, int fd, uint32_t size) {
    char buffer[256];
    snprintf(buffer,
        sizeof(buffer),
        "keyboard_handle_keymap: format=%u fd=%d size=%u",
        format,
        fd,
        size);
    rwl_log(RWL_LOG_DEBUG, buffer);

    if (format != WL_KEYBOARD_KEYMAP_FORMAT_XKB_V1) {
        snprintf(buffer, sizeof(buffer), "keyboard_handle_keymap: unsupported format=%u", format);
        rwl_log(RWL_LOG_ERROR, buffer);

        close(fd);
        return;
    }

    auto keymap_data = mmap(NULL, size, PROT_READ, MAP_PRIVATE, fd, 0);
    if (keymap_data == MAP_FAILED) {
        rwl_log(RWL_LOG_ERROR, "Failed to mmap keymap data");
        close(fd);
        return;
    }

    g_xkb_keymap = xkb_keymap_new_from_string(g_xkb_context,
        static_cast<const char*>(keymap_data),
        XKB_KEYMAP_FORMAT_TEXT_V1,
        XKB_KEYMAP_COMPILE_NO_FLAGS);

    munmap(keymap_data, size);
    close(fd);

    if (!g_xkb_keymap) {
        rwl_log(RWL_LOG_ERROR, "Failed to create XKB keymap");
        return;
    }

    g_xkb_state = xkb_state_new(g_xkb_keymap);
    if (!g_xkb_state) {
        rwl_log(RWL_LOG_ERROR, "Failed to create XKB state");
        return;
    }

    const char* locale = getenv("LC_ALL");
    if (!locale || !*locale)
        locale = getenv("LC_CTYPE");
    if (!locale || !*locale)
        locale = getenv("LANG");
    if (!locale || !*locale)
        locale = "C";

    auto compose_table =
        xkb_compose_table_new_from_locale(g_xkb_context, locale, XKB_COMPOSE_COMPILE_NO_FLAGS);
    if (compose_table) {
        g_xkb_compose_state = xkb_compose_state_new(compose_table, XKB_COMPOSE_STATE_NO_FLAGS);
        xkb_compose_table_unref(compose_table);
        if (!g_xkb_compose_state) {
            rwl_log(RWL_LOG_ERROR, "Failed to create XKB compose state");
            return;
        }
    } else {
        rwl_log(RWL_LOG_DEBUG, "No XKB compose table created");
    }
    g_xkb_mod_ctrl = xkb_keymap_mod_get_index(g_xkb_keymap, "Control");
    g_xkb_mod_alt = xkb_keymap_mod_get_index(g_xkb_keymap, "Mod1");
    g_xkb_mod_shift = xkb_keymap_mod_get_index(g_xkb_keymap, "Shift");
    g_xkb_mod_super = xkb_keymap_mod_get_index(g_xkb_keymap, "Mod4");
}

static void keyboard_handle_enter(void* data,
    struct wl_keyboard* keyboard,
    uint32_t serial,
    struct wl_surface* surface,
    struct wl_array* keys) {
    char buffer[256];
    snprintf(buffer, sizeof(buffer), "keyboard_handle_enter: serial=%u", serial);
    rwl_log(RWL_LOG_DEBUG, buffer);

    // Just in case
    if (!surface) {
        return;
    }

    rwl_window_internal* window =
        reinterpret_cast<rwl_window_internal*>(wl_surface_get_user_data(surface));

    if (window->surface != surface) {
        return;
    }

    g_window_with_keyboard = window;
}

static void keyboard_handle_leave(
    void* data, struct wl_keyboard* keyboard, uint32_t serial, struct wl_surface* surface) {
    char buffer[256];
    snprintf(buffer, sizeof(buffer), "keyboard_handle_leave: serial=%u", serial);
    rwl_log(RWL_LOG_DEBUG, buffer);

    // Just in case
    if (!surface) {
        return;
    }

    rwl_window_internal* window =
        reinterpret_cast<rwl_window_internal*>(wl_surface_get_user_data(surface));

    // Reset the window key states
    for (int i = 0; i < RWL_KEY_COUNT; ++i) {
        if (window->key_states[i]) {
            window->key_states[i] = false;

            if (window->key_callback) {
                window->key_callback(reinterpret_cast<rwl_window*>(window),
                    static_cast<rwl_key>(i),
                    0,
                    RWL_ACTION_RELEASE,
                    0);
            }
        }
    }

    g_mods = RWL_MOD_NONE;
    g_window_with_keyboard = nullptr;
}

static void keyboard_handle_key(void* data,
    struct wl_keyboard* keyboard,
    uint32_t serial,
    uint32_t time,
    uint32_t scancode,
    uint32_t state) {
    char buffer[512];
    snprintf(buffer,
        sizeof(buffer),
        "keyboard_handle_key: serial=%u time=%u scancode=%u state=%u",
        serial,
        time,
        scancode,
        state);
    rwl_log(RWL_LOG_DEBUG, buffer);

    rwl_window_internal* window = g_window_with_keyboard;
    if (!window) {
        return;
    }

    rwl_key key = translate_key(scancode);
    rwl_action action =
        (state == WL_KEYBOARD_KEY_STATE_PRESSED) ? RWL_ACTION_PRESS : RWL_ACTION_RELEASE;

    if (action == RWL_ACTION_RELEASE) {
        window->key_states[key] = false;
    } else if (action == RWL_ACTION_PRESS) {
        window->key_states[key] = true;

        // Emit character event on press if there's a character callback
        if (window->char_callback && g_xkb_state) {
            // Get the keysym from the state
            // +8 = offset for evdev keycodes, basically Wayland -> XKB conversion
            xkb_keysym_t keysym = xkb_state_key_get_one_sym(g_xkb_state, scancode + 8);
            // Convert keysym to UTF-32 codepoint
            uint32_t codepoint = xkb_keysym_to_utf32(keysym);
            if (codepoint != 0) {
                window->char_callback(reinterpret_cast<rwl_window*>(window), codepoint);
            }
        }
    }

    if (window->key_callback) {
        window->key_callback(reinterpret_cast<rwl_window*>(window), key, scancode, action, g_mods);
    }
}

static void keyboard_handle_modifiers(void* data,
    struct wl_keyboard* keyboard,
    uint32_t serial,
    uint32_t mods_depressed,
    uint32_t mods_latched,
    uint32_t mods_locked,
    uint32_t group) {
    if (!g_xkb_state) {
        return;
    }

    xkb_state_update_mask(g_xkb_state, mods_depressed, mods_latched, mods_locked, 0, 0, group);
    g_mods = RWL_MOD_NONE;

    if (xkb_state_mod_name_is_active(g_xkb_state, XKB_MOD_NAME_SHIFT, XKB_STATE_MODS_EFFECTIVE)) {
        g_mods = static_cast<rwl_mod>(g_mods | RWL_MOD_SHIFT);
    }
    if (xkb_state_mod_name_is_active(g_xkb_state, XKB_MOD_NAME_CTRL, XKB_STATE_MODS_EFFECTIVE)) {
        g_mods = static_cast<rwl_mod>(g_mods | RWL_MOD_CTRL);
    }
    if (xkb_state_mod_name_is_active(g_xkb_state, XKB_MOD_NAME_ALT, XKB_STATE_MODS_EFFECTIVE)) {
        g_mods = static_cast<rwl_mod>(g_mods | RWL_MOD_ALT);
    }
    if (xkb_state_mod_name_is_active(g_xkb_state, XKB_MOD_NAME_LOGO, XKB_STATE_MODS_EFFECTIVE)) {
        g_mods = static_cast<rwl_mod>(g_mods | RWL_MOD_SUPER);
    }
}

static void keyboard_handle_repeat_info(
    void* data, struct wl_keyboard* keyboard, int32_t rate, int32_t delay) {}

static const struct wl_keyboard_listener keyboard_listener{keyboard_handle_keymap,
    keyboard_handle_enter,
    keyboard_handle_leave,
    keyboard_handle_key,
    keyboard_handle_modifiers,
    keyboard_handle_repeat_info};

static void seat_handle_capabilities(void* data, struct wl_seat* seat, uint32_t capabilities) {
    char buffer[256];
    snprintf(buffer, sizeof(buffer), "seat_handle_capabilities: capabilities=0x%x", capabilities);
    rwl_log(RWL_LOG_DEBUG, buffer);

    if ((capabilities & WL_SEAT_CAPABILITY_KEYBOARD) && !g_keyboard) {
        g_keyboard = wl_seat_get_keyboard(seat);
        wl_keyboard_add_listener(g_keyboard, &keyboard_listener, nullptr);
    }
    if (!(capabilities & WL_SEAT_CAPABILITY_KEYBOARD) && g_keyboard) {
        wl_keyboard_destroy(g_keyboard);
        g_keyboard = nullptr;
    }
}
static void seat_handle_name(void* data, struct wl_seat* seat, const char* name) {
    // Handle seat name here
}

static const struct wl_seat_listener seat_listener = {
    seat_handle_capabilities,
    seat_handle_name,
};

// Registry listener callback
static void registry_global(void* data,
    struct wl_registry* registry,
    uint32_t name,
    const char* interface,
    uint32_t version) {
    char buffer[256];
    snprintf(buffer,
        sizeof(buffer),
        "registry_global: %s (name=%u version=%u)",
        interface,
        name,
        version);
    rwl_log(RWL_LOG_DEBUG, buffer);

    if (strcmp(interface, wl_compositor_interface.name) == 0) {
        g_compositor = static_cast<wl_compositor*>(
            wl_registry_bind(registry, name, &wl_compositor_interface, std::min(version, 4u)));
        rwl_log(RWL_LOG_WARNING, "Binding wl_compositor");
    } else if (strcmp(interface, zwlr_layer_shell_v1_interface.name) == 0) {
        g_layer_shell = static_cast<zwlr_layer_shell_v1*>(wl_registry_bind(
            registry, name, &zwlr_layer_shell_v1_interface, std::min(version, 4u)));
        rwl_log(RWL_LOG_WARNING, "Binding zwlr_layer_shell_v1");
    } else if (strcmp(interface, wp_fractional_scale_manager_v1_interface.name) == 0) {
        g_fractional_scale_manager = static_cast<wp_fractional_scale_manager_v1*>(wl_registry_bind(
            registry, name, &wp_fractional_scale_manager_v1_interface, std::min(version, 1u)));
        rwl_log(RWL_LOG_WARNING, "Binding wp_fractional_scale_manager_v1");
    } else if (strcmp(interface, wl_output_interface.name) == 0) {
        auto info = std::make_unique<rwl_output_info>();
        info->wl_name = name;
        info->scale = 1;  // Default scale
        info->output = static_cast<wl_output*>(
            wl_registry_bind(registry, name, &wl_output_interface, std::min(version, 4u)));

        rwl_output_info* info_ptr = info.get();
        g_outputs.push_back(std::move(info));
        wl_output_add_listener(info_ptr->output, &output_listener, info_ptr);

        char buffer[256];
        snprintf(buffer, sizeof(buffer), "Binding wl_output (name=%u)", name);
        rwl_log(RWL_LOG_WARNING, buffer);
    }
    // XDG
    else if ((strcmp(interface, xdg_wm_base_interface.name) == 0)) {
        g_xdg_wm_base = static_cast<xdg_wm_base*>(
            wl_registry_bind(registry, name, &xdg_wm_base_interface, std::min(version, 2u)));
        xdg_wm_base_add_listener(g_xdg_wm_base, &wm_base_listener, nullptr);
        rwl_log(RWL_LOG_WARNING, "Binding xdg_wm_base");
    } else if (strcmp(interface, wl_seat_interface.name) == 0) {
        g_seat = static_cast<wl_seat*>(
            wl_registry_bind(registry, name, &wl_seat_interface, std::min(version, 5u)));
        wl_seat_add_listener(g_seat, &seat_listener, nullptr);
        rwl_log(RWL_LOG_WARNING, "Binding wl_seat");
    }
}

static void registry_remove(void* data, struct wl_registry* registry, uint32_t name) {
    char buffer[256];
    snprintf(buffer, sizeof(buffer), "registry_remove: name=%u", name);
    rwl_log(RWL_LOG_WARNING, buffer);

    // Remove output if it matches
    for (auto it = g_outputs.begin(); it != g_outputs.end(); ++it) {
        if ((*it)->wl_name == name) {
            snprintf(buffer,
                sizeof(buffer),
                "Removing output: %s",
                (*it)->name.empty() ? "(unknown)" : (*it)->name.c_str());
            rwl_log(RWL_LOG_INFO, buffer);

            if ((*it)->output) {
                wl_output_destroy((*it)->output);
            }
            g_outputs.erase(it);
            break;
        }
    }
}

static const struct wl_registry_listener registry_listener = {
    registry_global,
    registry_remove,
};

static void xdg_surface_configure(void* data, struct xdg_surface* surface, uint32_t serial) {
    xdg_surface_ack_configure(surface, serial);
}

static const struct xdg_surface_listener xdg_surface_listener = {
    xdg_surface_configure,
};

static void xdg_toplevel_handle_configure(void* data,
    struct xdg_toplevel* toplevel,
    int32_t width,
    int32_t height,
    struct wl_array* states) {
    rwl_window_internal* window = reinterpret_cast<rwl_window_internal*>(data);

    char buffer[256];
    snprintf(
        buffer, sizeof(buffer), "xdg_toplevel_handle_configure: width=%u height=%u", width, height);
    rwl_log(RWL_LOG_DEBUG, buffer);

    bool size_is_changing = (window->width != width) || (window->height != height);
    window->width = width;
    window->height = height;

    if (!size_is_changing) {
        return;
    }

    // Call logical size callback
    if (window->logical_size_callback) {
        window->logical_size_callback(reinterpret_cast<rwl_window*>(window), width, height);
    }

    // Calculate and call pixel size callback
    if (window->pixel_size_callback) {
        uint32_t pixel_width, pixel_height;
        if (window->preferred_fractional_scale != 120) {
            pixel_width = (width * window->preferred_fractional_scale + 60) / 120;
            pixel_height = (height * window->preferred_fractional_scale + 60) / 120;
        } else {
            pixel_width = width;
            pixel_height = height;
        }
        window->pixel_size_callback(
            reinterpret_cast<rwl_window*>(window), pixel_width, pixel_height);
    }
}

static void xdg_toplevel_handle_close(void* data, struct xdg_toplevel* toplevel) {
    rwl_window_internal* window = reinterpret_cast<rwl_window_internal*>(data);
    window->should_close = true;
}

static const struct xdg_toplevel_listener xdg_toplevel_listener = {
    xdg_toplevel_handle_configure, xdg_toplevel_handle_close};

void rwlSetLogCallback(rwl_log_severity min_severity, rwl_log_callback callback) {
    g_log_severity = min_severity;
    g_log_callback = callback;
}

rwl_status rwlStartup() {
    if (g_display) {
        rwl_log(RWL_LOG_WARNING, "rwlStartup called but already initialized");
        return RWL_STATUS_ALREADY_INITIALIZED;
    }

    // ========== XKB
    {
        create_key_tables();

        g_xkb_context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
        if (!g_xkb_context) {
            rwl_log(RWL_LOG_ERROR, "Failed to create XKB context");
            return RWL_STATUS_INTERNAL_ERROR;
        }
    }

    g_wake_fd = eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC);
    if (g_wake_fd == -1) {
        rwl_log(RWL_LOG_ERROR, "Failed to create wake eventfd");
        return RWL_STATUS_INTERNAL_ERROR;
    }

    g_display = wl_display_connect(nullptr);
    if (!g_display) {
        close(g_wake_fd);
        g_wake_fd = -1;
        rwl_log(RWL_LOG_ERROR, "Failed to connect to Wayland display");
        return RWL_STATUS_NO_DISPLAY;
    }
    rwl_log(RWL_LOG_DEBUG, "Connected to Wayland display");

    g_registry = wl_display_get_registry(g_display);
    if (!g_registry) {
        rwl_log(RWL_LOG_ERROR, "Failed to get registry");
        wl_display_disconnect(g_display);
        g_display = nullptr;
        return RWL_STATUS_NO_REGISTRY;
    }

    // Attach listener and sync to receive all globals
    wl_registry_add_listener(g_registry, &registry_listener, nullptr);
    wl_display_roundtrip(g_display);

    // Check if we got the required globals
    if (!g_compositor) {
        rwl_log(RWL_LOG_ERROR, "Compositor not available");
        wl_registry_destroy(g_registry);
        wl_display_disconnect(g_display);
        g_display = nullptr;
        g_registry = nullptr;
        return RWL_STATUS_NO_COMPOSITOR;
    }

    if (!g_layer_shell) {
        rwl_log(RWL_LOG_ERROR, "Layer shell not available");
        wl_registry_destroy(g_registry);
        wl_display_disconnect(g_display);
        g_display = nullptr;
        g_registry = nullptr;
        return RWL_STATUS_NO_LAYER_SHELL;
    }

    // Another roundtrip to missing information from
    // globals that were bound during the first roundtrip
    wl_display_roundtrip(g_display);

    return RWL_STATUS_OK;
}

rwl_status rwlShutdown() {
    if (!g_display) {
        rwl_log(RWL_LOG_WARNING, "rwlShutdown called but not initialized");
        return RWL_STATUS_NOT_INITIALIZED;
    }

    g_window_with_keyboard = nullptr;

    // ========== XKB
    {
        if (g_xkb_compose_state) {
            xkb_compose_state_unref(g_xkb_compose_state);
            g_xkb_compose_state = nullptr;
        }

        if (g_xkb_state) {
            xkb_state_unref(g_xkb_state);
            g_xkb_state = nullptr;
        }

        if (g_xkb_keymap) {
            xkb_keymap_unref(g_xkb_keymap);
            g_xkb_keymap = nullptr;
        }

        if (g_xkb_context) {
            xkb_context_unref(g_xkb_context);
            g_xkb_context = nullptr;
        }
    }

    if (g_keyboard) {
        wl_keyboard_destroy(g_keyboard);
        g_keyboard = nullptr;
    }

    if (g_seat) {
        wl_seat_destroy(g_seat);
        g_seat = nullptr;
    }

    if (g_xdg_wm_base) {
        xdg_wm_base_destroy(g_xdg_wm_base);
        g_xdg_wm_base = nullptr;
    }

    // Clean up outputs
    for (auto& output : g_outputs) {
        if (output->output) {
            wl_output_destroy(output->output);
        }
    }
    g_outputs.clear();

    if (g_fractional_scale_manager) {
        wp_fractional_scale_manager_v1_destroy(g_fractional_scale_manager);
        g_fractional_scale_manager = nullptr;
    }

    if (g_layer_shell) {
        zwlr_layer_shell_v1_destroy(g_layer_shell);
        g_layer_shell = nullptr;
    }

    if (g_compositor) {
        wl_compositor_destroy(g_compositor);
        g_compositor = nullptr;
    }

    if (g_registry) {
        wl_registry_destroy(g_registry);
        g_registry = nullptr;
    }
    if (g_display) {
        wl_display_disconnect(g_display);
        g_display = nullptr;
    }

    if (g_wake_fd != -1) {
        close(g_wake_fd);
        g_wake_fd = -1;
    }

    return RWL_STATUS_OK;
}

rwl_status rwlCreateWindow(rwl_window_type type,
    wl_output* output,
    uint32_t width,
    uint32_t height,
    rwl_window** window_out) {
    if (!window_out) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }
    *window_out = nullptr;

    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    if (!g_compositor) {
        return RWL_STATUS_NO_COMPOSITOR;
    }

    // Allocate window structure
    auto window = std::make_unique<rwl_window_internal>();

    window->output = output;
    window->width = width;
    window->height = height;
    window->fractional_scale = nullptr;
    window->preferred_fractional_scale = 120;  // Default 1.0x (120/120)

    // Create underlying wl_surface
    window->surface = wl_compositor_create_surface(g_compositor);
    if (!window->surface) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    wl_surface_set_user_data(window->surface, window.get());

    // Create fractional scale object if manager is available
    if (g_fractional_scale_manager) {
        window->fractional_scale = wp_fractional_scale_manager_v1_get_fractional_scale(
            g_fractional_scale_manager, window->surface);
        if (window->fractional_scale) {
            wp_fractional_scale_v1_add_listener(
                window->fractional_scale, &window_fractional_scale_listener, window.get());
            rwl_log(RWL_LOG_DEBUG, "Created fractional scale object for window");
        }
    }

    bool is_layer_shell_window =
        (type == RWL_WINDOW_TYPE_BACKGROUND || type == RWL_WINDOW_TYPE_TASKBAR);

    // Create layer surface if layer shell is available
    if (is_layer_shell_window && g_layer_shell) {
        // Determine layer and anchor based on window type
        uint32_t layer;
        uint32_t anchor;
        int32_t exclusive_zone;

        if (type == RWL_WINDOW_TYPE_BACKGROUND) {
            // Background: bottom layer, covers full screen
            layer = ZWLR_LAYER_SHELL_V1_LAYER_BACKGROUND;
            anchor = ZWLR_LAYER_SURFACE_V1_ANCHOR_TOP | ZWLR_LAYER_SURFACE_V1_ANCHOR_RIGHT |
                     ZWLR_LAYER_SURFACE_V1_ANCHOR_BOTTOM | ZWLR_LAYER_SURFACE_V1_ANCHOR_LEFT;
            exclusive_zone = -1;  // Background doesn't occlude
        } else if (type == RWL_WINDOW_TYPE_TASKBAR) {
            // Taskbar: top layer, anchored to top, reserves vertical space
            layer = ZWLR_LAYER_SHELL_V1_LAYER_TOP;
            anchor = ZWLR_LAYER_SURFACE_V1_ANCHOR_TOP | ZWLR_LAYER_SURFACE_V1_ANCHOR_LEFT |
                     ZWLR_LAYER_SURFACE_V1_ANCHOR_RIGHT;
            exclusive_zone = height;  // Reserve space for taskbar
        } else {
            wl_surface_destroy(window->surface);
            return RWL_STATUS_INVALID_ARGUMENT;
        }

        window->layer_surface = zwlr_layer_shell_v1_get_layer_surface(
            g_layer_shell, window->surface, output, layer, "rustine-wl");
        if (!window->layer_surface) {
            wl_surface_destroy(window->surface);
            return RWL_STATUS_INTERNAL_ERROR;
        }
        zwlr_layer_surface_v1_add_listener(
            window->layer_surface, &layer_surface_listener, window.get());
        zwlr_layer_surface_v1_set_size(window->layer_surface, width, height);
        zwlr_layer_surface_v1_set_anchor(window->layer_surface, anchor);
        zwlr_layer_surface_v1_set_exclusive_zone(window->layer_surface, exclusive_zone);
    } else {
        if (!g_xdg_wm_base) {
            rwl_log(RWL_LOG_ERROR, "xdg_wm_base is not available");
            return RWL_STATUS_INTERNAL_ERROR;
        }
        window->xdg_surface = xdg_wm_base_get_xdg_surface(g_xdg_wm_base, window->surface);
        if (!window->xdg_surface) {
            wl_surface_destroy(window->surface);
            return RWL_STATUS_INTERNAL_ERROR;
        }
        xdg_surface_add_listener(window->xdg_surface, &xdg_surface_listener, window.get());
        window->xdg_toplevel = xdg_surface_get_toplevel(window->xdg_surface);
        if (!window->xdg_toplevel) {
            xdg_surface_destroy(window->xdg_surface);
            wl_surface_destroy(window->surface);
            return RWL_STATUS_INTERNAL_ERROR;
        }
        xdg_toplevel_add_listener(window->xdg_toplevel, &xdg_toplevel_listener, window.get());
        xdg_toplevel_set_title(window->xdg_toplevel, "rustine-wl");
    }

    wl_surface_commit(window->surface);
    wl_display_roundtrip(g_display);  // Wait for configure

    *window_out = reinterpret_cast<rwl_window*>(window.get());
    g_windows.push_back(std::move(window));
    return RWL_STATUS_OK;
}

rwl_status rwlDestroyWindow(rwl_window* window) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    rwl_cleanup_window(win);

    for (auto it = g_windows.begin(); it != g_windows.end(); ++it) {
        if (it->get() == win) {
            g_windows.erase(it);
            break;
        }
    }
    return RWL_STATUS_OK;
}

rwl_status rwlSetWindowUserPointer(rwl_window* window, void* pointer) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    win->user_pointer = pointer;
    return RWL_STATUS_OK;
}

void* rwlGetWindowUserPointer(rwl_window* window) {
    if (!window) {
        return nullptr;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    return win->user_pointer;
}

rwl_status rwlSetPixelSizeCallback(rwl_window* window, rwl_pixel_size_callback callback) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    win->pixel_size_callback = callback;

    return RWL_STATUS_OK;
}

rwl_status rwlSetLogicalSizeCallback(rwl_window* window, rwl_logical_size_callback callback) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    win->logical_size_callback = callback;

    return RWL_STATUS_OK;
}

rwl_status rwlSetKeyCallback(rwl_window* window, rwl_key_callback callback) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    win->key_callback = callback;

    return RWL_STATUS_OK;
}

rwl_status rwlSetCharCallback(rwl_window* window, rwl_char_callback callback) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    win->char_callback = callback;

    return RWL_STATUS_OK;
}

rwl_status rwlGetWaylandHandles(
    rwl_window* window, wl_display** out_display, wl_surface** out_surface) {
    if (!window || !out_display || !out_surface) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    *out_display = nullptr;
    *out_surface = nullptr;

    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    if (!win->surface) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    *out_display = g_display;
    *out_surface = win->surface;

    return RWL_STATUS_OK;
}

rwl_status rwlGetPixelSize(rwl_window* window, uint32_t* width, uint32_t* height) {
    if (!window || !width || !height) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);

    // Calculate buffer size based on fractional scale
    // Fractional scale has denominator 120, so 150 = 1.25x
    // Round half-up: (size * scale + 60) / 120
    if (win->preferred_fractional_scale != 120) {
        *width = (win->width * win->preferred_fractional_scale + 60) / 120;
        *height = (win->height * win->preferred_fractional_scale + 60) / 120;
    } else {
        // No fractional scaling, use logical size directly
        *width = win->width;
        *height = win->height;
    }

    return RWL_STATUS_OK;
}

rwl_status rwlGetLogicalSize(rwl_window* window, uint32_t* width, uint32_t* height) {
    if (!window || !width || !height) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    *width = win->width;
    *height = win->height;

    return RWL_STATUS_OK;
}

// Internal helper to dispatch events with optional timeout (in milliseconds).
// timeout_ms < 0 blocks indefinitely.
static rwl_status rwl_wait_events_internal(int timeout_ms) {
    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    struct pollfd pfds[2];
    nfds_t nfds = 1;
    pfds[0].fd = wl_display_get_fd(g_display);
    pfds[0].events = POLLIN;
    pfds[0].revents = 0;

    if (g_wake_fd != -1) {
        pfds[1].fd = g_wake_fd;
        pfds[1].events = POLLIN;
        pfds[1].revents = 0;
        nfds = 2;
    }

    // Prepare for reading; if there's pending work, dispatch it first.
    while (wl_display_prepare_read(g_display) != 0) {
        if (wl_display_dispatch_pending(g_display) == -1) {
            return RWL_STATUS_INTERNAL_ERROR;
        }
    }

    int flush_result;
    while ((flush_result = wl_display_flush(g_display)) == -1 && errno == EAGAIN) {
        struct pollfd fd = {wl_display_get_fd(g_display), POLLOUT, 0};
        if (poll(&fd, 1, -1) == -1 && errno != EINTR) {
            wl_display_cancel_read(g_display);
            rwl_request_close_all_windows();
            return RWL_STATUS_INTERNAL_ERROR;
        }
    }

    if (flush_result == -1) {
        wl_display_cancel_read(g_display);
        rwl_log(RWL_LOG_ERROR, "Wayland connection error during flush; requesting window close");
        rwl_request_close_all_windows();
        return RWL_STATUS_INTERNAL_ERROR;
    }

    int poll_result;
    do {
        poll_result = poll(pfds, nfds, timeout_ms);
    } while (poll_result == -1 && errno == EINTR);

    bool display_ready = false;
    bool wake_ready = false;
    if (poll_result > 0) {
        display_ready = (pfds[0].revents & POLLIN) != 0;
        wake_ready = (nfds > 1) && (pfds[1].revents & POLLIN);
    }

    if (display_ready) {
        if (wl_display_read_events(g_display) == -1) {
            return RWL_STATUS_INTERNAL_ERROR;
        }
        if (wl_display_dispatch_pending(g_display) == -1) {
            return RWL_STATUS_INTERNAL_ERROR;
        }
    } else {
        wl_display_cancel_read(g_display);
    }

    if (wake_ready) {
        uint64_t value;
        while (read(g_wake_fd, &value, sizeof(value)) == sizeof(value)) {}
    }

    if (poll_result == -1) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    return RWL_STATUS_OK;
}

rwl_status rwlPollEvents() {
    // Non-blocking: dispatch what is ready and return.
    return rwl_wait_events_internal(0);
}

rwl_status rwlWaitEvents() {
    // Block until an event is available.
    return rwl_wait_events_internal(-1);
}

rwl_status rwlWaitEventsTimeout(uint64_t timeout_ns) {
    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    // Vulkan uses UINT64_MAX for infinite
    if (timeout_ns == std::numeric_limits<uint64_t>::max()) {
        return rwlWaitEvents();
    }

    // Convert nanoseconds to milliseconds, rounding up to avoid premature
    // timeouts.
    uint64_t ms_u64 = (timeout_ns + 999999ull) / 1000000ull;
    int timeout_ms;
    if (ms_u64 > static_cast<uint64_t>(std::numeric_limits<int>::max())) {
        timeout_ms = std::numeric_limits<int>::max();
    } else {
        timeout_ms = static_cast<int>(ms_u64);
    }

    return rwl_wait_events_internal(timeout_ms);
}

rwl_status rwlPostEmptyEvent() {
    if (g_wake_fd == -1) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    uint64_t value = 1;
    ssize_t written = write(g_wake_fd, &value, sizeof(value));
    if (written == -1 && errno != EAGAIN) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    return RWL_STATUS_OK;
}

bool rwlWindowShouldClose(rwl_window* window) {
    if (!window) {
        return false;
    }
    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    return win->should_close;
}

rwl_status rwlWindowRequestClose(rwl_window* window) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }
    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    win->should_close = true;
    return RWL_STATUS_OK;
}

// Output management functions
rwl_status rwlEnumerateOutputs(uint32_t* count, rwl_output_info_public* outputs_out) {
    if (!count) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }
    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    uint32_t output_count = static_cast<uint32_t>(g_outputs.size());

    if (outputs_out == nullptr) {
        // First call: just return the count
        *count = output_count;
        return RWL_STATUS_OK;
    }

    // Second call: fill in the output information
    if (*count < output_count) {
        *count = output_count;
        return RWL_STATUS_INVALID_ARGUMENT;  // Buffer too small
    }

    for (uint32_t i = 0; i < output_count; ++i) {
        const auto& output = g_outputs[i];
        outputs_out[i].wl_output = output->output;
        outputs_out[i].name = output->name.c_str();
        outputs_out[i].description = output->description.c_str();
        outputs_out[i].scale = output->scale;
        outputs_out[i].width = output->width;
        outputs_out[i].height = output->height;
    }

    *count = output_count;
    return RWL_STATUS_OK;
}

#ifdef __cplusplus
}
#endif