
cmake_minimum_required(VERSION 3.16)
project(rustine_webp LANGUAGES C CXX)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(GNUInstallDirs)

find_package(PkgConfig REQUIRED)

# Prefer using system-provided libwebp (via pkg-config). If `libwebpdemux` is
# available as a separate pkg-config module, prefer linking it too.
pkg_check_modules(WEBP REQUIRED libwebp)
pkg_check_modules(WEBPDEMUX QUIET libwebpdemux)

add_library(rustine_webp STATIC rustine-webp.cpp)
set_target_properties(rustine_webp PROPERTIES OUTPUT_NAME "rustine_webp")

target_include_directories(
	rustine_webp
	PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}
		   ${WEBP_INCLUDE_DIRS})

if (WEBPDEMUX_FOUND)
	target_include_directories(rustine_webp PUBLIC ${WEBPDEMUX_INCLUDE_DIRS})
	# Link webpdemux before webp to ensure libwebp symbols are available
	# when linking static webpdemux objects that reference them.
	target_link_libraries(rustine_webp PUBLIC ${WEBPDEMUX_LIBRARIES} ${WEBP_LIBRARIES})
else()
	target_link_libraries(rustine_webp PUBLIC ${WEBP_LIBRARIES})
endif()

install(
	TARGETS rustine_webp
	ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
	LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
	RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
	INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})

install(FILES rustine-webp.hpp DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
