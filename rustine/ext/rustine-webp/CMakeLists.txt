
cmake_minimum_required(VERSION 3.16)
project(rustine_webp LANGUAGES C CXX)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(GNUInstallDirs)

# Build libwebp as a static dependency without extra tools.
set(BUILD_SHARED_LIBS OFF CACHE BOOL "Build shared libraries" FORCE)
set(WEBP_LINK_STATIC ON CACHE BOOL "Link libwebp statically" FORCE)
set(WEBP_BUILD_ANIM_UTILS OFF CACHE BOOL "Disable libwebp animation utils" FORCE)
set(WEBP_BUILD_CWEBP OFF CACHE BOOL "Disable libwebp cwebp tool" FORCE)
set(WEBP_BUILD_DWEBP OFF CACHE BOOL "Disable libwebp dwebp tool" FORCE)
set(WEBP_BUILD_GIF2WEBP OFF CACHE BOOL "Disable libwebp gif2webp tool" FORCE)
set(WEBP_BUILD_IMG2WEBP OFF CACHE BOOL "Disable libwebp img2webp tool" FORCE)
set(WEBP_BUILD_VWEBP OFF CACHE BOOL "Disable libwebp vwebp tool" FORCE)
set(WEBP_BUILD_WEBPINFO OFF CACHE BOOL "Disable libwebp webpinfo tool" FORCE)
set(WEBP_BUILD_LIBWEBPMUX OFF CACHE BOOL "Disable libwebpmux" FORCE)
set(WEBP_BUILD_WEBPMUX OFF CACHE BOOL "Disable libwebpmux tool" FORCE)
set(WEBP_BUILD_EXTRAS OFF CACHE BOOL "Disable libwebp extras" FORCE)
set(WEBP_BUILD_WEBP_JS OFF CACHE BOOL "Disable libwebp webp.js" FORCE)
set(WEBP_BUILD_FUZZTEST OFF CACHE BOOL "Disable libwebp fuzz tests" FORCE)
set(WEBP_USE_THREAD ON CACHE BOOL "Enable libwebp threading" FORCE)
set(WEBP_ENABLE_SIMD ON CACHE BOOL "Enable libwebp SIMD" FORCE)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../libwebp ${CMAKE_CURRENT_BINARY_DIR}/libwebp)

add_library(rustine_webp STATIC rustine-webp.cpp)
set_target_properties(rustine_webp PROPERTIES OUTPUT_NAME "rustine_webp")

target_include_directories(
	rustine_webp
	PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}
				 ${CMAKE_CURRENT_SOURCE_DIR}/../libwebp/src)

target_link_libraries(rustine_webp PUBLIC webp webpdemux)

install(
	TARGETS rustine_webp
	ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
	LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
	RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
	INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})

install(FILES rustine-webp.hpp DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
