cmake_minimum_required(VERSION 3.16)
project(rustine_wl LANGUAGES C CXX)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(GNUInstallDirs)

# Find required dependencies
find_package(PkgConfig REQUIRED)
pkg_check_modules(WAYLAND_CLIENT REQUIRED IMPORTED_TARGET wayland-client)
pkg_check_modules(WAYLAND_PROTOCOLS REQUIRED IMPORTED_TARGET wayland-protocols)
pkg_get_variable(WAYLAND_PROTOCOLS_DIR wayland-protocols pkgdatadir)
pkg_get_variable(WAYLAND_SCANNER wayland-scanner wayland_scanner)

# Define the protocol files
set(PROTOCOLS
    ${WAYLAND_PROTOCOLS_DIR}/stable/xdg-shell/xdg-shell.xml
    ${WAYLAND_PROTOCOLS_DIR}/stable/viewporter/viewporter.xml
    ${WAYLAND_PROTOCOLS_DIR}/staging/single-pixel-buffer/single-pixel-buffer-v1.xml
    ${WAYLAND_PROTOCOLS_DIR}/staging/fractional-scale/fractional-scale-v1.xml
    ${CMAKE_CURRENT_SOURCE_DIR}/wlr-layer-shell-unstable-v1.xml
)

# Generate protocol sources
set(PROTOCOL_GENERATED_SOURCES "")
foreach(PROTOCOL ${PROTOCOLS})
    get_filename_component(PROTOCOL_NAME ${PROTOCOL} NAME_WE)
    
    # Generate private-code
    set(PROTOCOL_C "${CMAKE_CURRENT_BINARY_DIR}/${PROTOCOL_NAME}-protocol.c")
    add_custom_command(
        OUTPUT ${PROTOCOL_C}
        COMMAND ${WAYLAND_SCANNER} private-code ${PROTOCOL} ${PROTOCOL_C}
        DEPENDS ${PROTOCOL}
        VERBATIM
    )
    
    # Generate client-header
    set(PROTOCOL_H "${CMAKE_CURRENT_BINARY_DIR}/${PROTOCOL_NAME}-client-protocol.h")
    add_custom_command(
        OUTPUT ${PROTOCOL_H}
        COMMAND ${WAYLAND_SCANNER} client-header ${PROTOCOL} ${PROTOCOL_H}
        DEPENDS ${PROTOCOL}
        VERBATIM
    )
    
    list(APPEND PROTOCOL_GENERATED_SOURCES ${PROTOCOL_C} ${PROTOCOL_H})
endforeach()

# Create the rustine-wl static library
add_library(rustine_wl STATIC
    rustine-wl.cpp
    ${PROTOCOL_GENERATED_SOURCES}
)

set_target_properties(rustine_wl PROPERTIES OUTPUT_NAME "rustine-wl")

target_include_directories(rustine_wl
    PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}
    PRIVATE ${CMAKE_CURRENT_BINARY_DIR}
)

target_link_libraries(rustine_wl PUBLIC PkgConfig::WAYLAND_CLIENT)

install(
    TARGETS rustine_wl
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

install(FILES rustine-wl.hpp DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
