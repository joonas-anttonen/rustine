#include "rustine-wl.hpp"

#include <cstring>
#include <stdexcept>
#include <utility>

static wl_display* g_display = nullptr;
static wl_registry* g_registry = nullptr;
static wl_compositor* g_compositor = nullptr;
static zwlr_layer_shell_v1* g_layer_shell = nullptr;
static bool g_should_close = false;

// Internal window structure
struct rwl_window_internal {
    wl_surface* surface;
    wl_output* output;
    zwlr_layer_surface_v1* layer_surface;
    uint32_t width;
    uint32_t height;
    rwl_framebuffer_size_callback size_callback;
    rwl_frame_callback frame_callback;
    wl_callback* frame_cb;
};

#ifdef __cplusplus
extern "C" {
#endif

// Forward declare frame_done for listener
static void frame_done(void* data, struct wl_callback* callback, uint32_t time);

static const struct wl_callback_listener frame_listener = {
    frame_done,
};

// Frame callback listener
static void frame_done(void* data, struct wl_callback* callback, uint32_t time) {
    rwl_window_internal* window = static_cast<rwl_window_internal*>(data);
    
    // Destroy old callback
    if (callback) {
        wl_callback_destroy(callback);
    }
    window->frame_cb = nullptr;
    
    // Fire user callback
    if (window->frame_callback) {
        window->frame_callback(reinterpret_cast<rwl_window*>(window));
    }
    
    // Auto-renew frame callback if user callback is still set
    if (window->frame_callback && window->surface) {
        window->frame_cb = wl_surface_frame(window->surface);
        wl_callback_add_listener(window->frame_cb, &frame_listener, window);
    }
}

// Layer surface listener callback
static void layer_surface_handle_configure(void* data, struct zwlr_layer_surface_v1* surface,
                                           uint32_t serial, uint32_t width, uint32_t height) {
    rwl_window_internal* window = static_cast<rwl_window_internal*>(data);
    window->width = width;
    window->height = height;
    zwlr_layer_surface_v1_ack_configure(surface, serial);
    
    if (window->size_callback) {
        window->size_callback(reinterpret_cast<rwl_window*>(window), width, height);
    }
}

static void layer_surface_handle_closed(void* data, struct zwlr_layer_surface_v1* surface) {
    // Compositor closed the layer surface; set global close flag
    g_should_close = true;
}

static const struct zwlr_layer_surface_v1_listener layer_surface_listener = {
    layer_surface_handle_configure,
    layer_surface_handle_closed,
};

// Registry listener callback
static void registry_handle_global(void* data, struct wl_registry* registry,
                                  uint32_t name, const char* interface, uint32_t version) {
    if (strcmp(interface, wl_compositor_interface.name) == 0) {
        g_compositor = static_cast<wl_compositor*>(
            wl_registry_bind(registry, name, &wl_compositor_interface, std::min(version, 4u)));
    } else if (strcmp(interface, zwlr_layer_shell_v1_interface.name) == 0) {
        g_layer_shell = static_cast<zwlr_layer_shell_v1*>(
            wl_registry_bind(registry, name, &zwlr_layer_shell_v1_interface, std::min(version, 4u)));
    }
}

static void registry_handle_global_remove(void* data, struct wl_registry* registry, uint32_t name) {
    // Global removed; if it's our compositor or layer shell, we'd need to handle it.
    // For now, just log or ignore.
}

static const struct wl_registry_listener registry_listener = {
    registry_handle_global,
    registry_handle_global_remove,
};

rwl_status rwlInit() {
    if (g_display) {
        return RWL_STATUS_ALREADY_INITIALIZED;
    }

    g_display = wl_display_connect(nullptr);
    if (!g_display) {
        return RWL_STATUS_NO_DISPLAY;
    }

    g_registry = wl_display_get_registry(g_display);
    if (!g_registry) {
        wl_display_disconnect(g_display);
        g_display = nullptr;
        return RWL_STATUS_NO_REGISTRY;
    }

    // Attach listener and sync to receive all globals
    wl_registry_add_listener(g_registry, &registry_listener, nullptr);
    wl_display_roundtrip(g_display);
    
    // Check if we got the required globals
    if (!g_compositor) {
        wl_registry_destroy(g_registry);
        wl_display_disconnect(g_display);
        g_display = nullptr;
        g_registry = nullptr;
        return RWL_STATUS_NO_COMPOSITOR;
    }

    if (!g_layer_shell) {
        // Layer shell is optional for now; we can continue without it for basic surfaces
        // but the user will be limited
    }
    
    g_should_close = false;
    return RWL_STATUS_OK;
}

rwl_status rwlShutdown() {
    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
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

    return RWL_STATUS_OK;
}

rwl_status rwlCreateWindow(rwl_window_type type, wl_output* output, uint32_t width, uint32_t height, rwl_window** window_out) {
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
    rwl_window_internal* window = new rwl_window_internal{};
    if (!window) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    window->output = output;
    window->width = width;
    window->height = height;

    // Create underlying wl_surface
    window->surface = wl_compositor_create_surface(g_compositor);
    if (!window->surface) {
        delete window;
        return RWL_STATUS_INTERNAL_ERROR;
    }

    // Create layer surface if layer shell is available
    if (g_layer_shell) {
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
            delete window;
            return RWL_STATUS_INVALID_ARGUMENT;
        }

        window->layer_surface = zwlr_layer_shell_v1_get_layer_surface(
            g_layer_shell, window->surface, output, layer, "rustine-wl");
        if (!window->layer_surface) {
            wl_surface_destroy(window->surface);
            delete window;
            return RWL_STATUS_INTERNAL_ERROR;
        }
        zwlr_layer_surface_v1_add_listener(window->layer_surface, &layer_surface_listener, window);
        zwlr_layer_surface_v1_set_size(window->layer_surface, width, height);
        zwlr_layer_surface_v1_set_anchor(window->layer_surface, anchor);
        zwlr_layer_surface_v1_set_exclusive_zone(window->layer_surface, exclusive_zone);
        wl_surface_commit(window->surface);
        wl_display_roundtrip(g_display);  // Wait for configure
    }

    *window_out = reinterpret_cast<rwl_window*>(window);
    return RWL_STATUS_OK;
}

rwl_status rwlDestroyWindow(rwl_window* window) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);

    if (win->frame_cb) {
        wl_callback_destroy(win->frame_cb);
    }

    if (win->layer_surface) {
        zwlr_layer_surface_v1_destroy(win->layer_surface);
    }

    if (win->surface) {
        wl_surface_destroy(win->surface);
    }

    delete win;
    return RWL_STATUS_OK;
}

rwl_status rwlSetFramebufferSizeCallback(rwl_window* window, rwl_framebuffer_size_callback callback) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    win->size_callback = callback;
    return RWL_STATUS_OK;
}

rwl_status rwlSetFrameCallback(rwl_window* window, rwl_frame_callback callback) {
    if (!window) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    
    // If clearing callback, destroy pending frame callback
    if (!callback && win->frame_cb) {
        wl_callback_destroy(win->frame_cb);
        win->frame_cb = nullptr;
    }
    
    win->frame_callback = callback;
    
    // If setting for the first time, request initial frame
    if (callback && !win->frame_cb && win->surface) {
        win->frame_cb = wl_surface_frame(win->surface);
        wl_callback_add_listener(win->frame_cb, &frame_listener, win);
    }
    
    return RWL_STATUS_OK;
}

rwl_status rwlCreateSurface(VkInstance instance, rwl_window* window, VkSurfaceKHR* surface_out) {
    if (!window || !surface_out) {
        return RWL_STATUS_INVALID_ARGUMENT;
    }
    *surface_out = VK_NULL_HANDLE;

    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    rwl_window_internal* win = reinterpret_cast<rwl_window_internal*>(window);
    if (!win->surface) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    // Get vkCreateWaylandSurfaceKHR function pointer
    PFN_vkCreateWaylandSurfaceKHR vkCreateWaylandSurfaceKHR =
        (PFN_vkCreateWaylandSurfaceKHR)vkGetInstanceProcAddr(instance, "vkCreateWaylandSurfaceKHR");
    if (!vkCreateWaylandSurfaceKHR) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    // Create Vulkan surface from window's wl_surface
    VkWaylandSurfaceCreateInfoKHR create_info = {};
    create_info.sType = VK_STRUCTURE_TYPE_WAYLAND_SURFACE_CREATE_INFO_KHR;
    create_info.display = g_display;
    create_info.surface = win->surface;

    VkResult result = vkCreateWaylandSurfaceKHR(instance, &create_info, nullptr, surface_out);
    if (result != VK_SUCCESS) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    return RWL_STATUS_OK;
}

rwl_status rwlProcessEvents() {
    if (!g_display) {
        return RWL_STATUS_NOT_INITIALIZED;
    }

    // Dispatch pending events without blocking
    if (wl_display_dispatch_pending(g_display) == -1) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    // Flush outgoing requests
    if (wl_display_flush(g_display) == -1) {
        return RWL_STATUS_INTERNAL_ERROR;
    }

    return RWL_STATUS_OK;
}

bool rwlShouldClose() {
    return g_should_close;
}

void rwlRequestClose() {
    g_should_close = true;
}

#ifdef __cplusplus
}
#endif