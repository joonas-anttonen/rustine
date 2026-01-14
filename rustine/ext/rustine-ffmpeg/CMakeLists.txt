cmake_minimum_required(VERSION 3.20)
project(rustine-ffmpeg LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find FFmpeg components
find_package(PkgConfig REQUIRED)
pkg_check_modules(AVCODEC REQUIRED IMPORTED_TARGET libavcodec)
pkg_check_modules(AVFORMAT REQUIRED IMPORTED_TARGET libavformat)
pkg_check_modules(AVUTIL REQUIRED IMPORTED_TARGET libavutil)
pkg_check_modules(SWSCALE REQUIRED IMPORTED_TARGET libswscale)

add_library(rustine-ffmpeg STATIC
	rustine-ffmpeg.cpp
	rustine-ffmpeg.hpp
)

target_include_directories(rustine-ffmpeg PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

target_link_libraries(rustine-ffmpeg
	PkgConfig::AVCODEC
	PkgConfig::AVFORMAT
	PkgConfig::AVUTIL
	PkgConfig::SWSCALE
)

# Installation
install(TARGETS rustine-ffmpeg
	ARCHIVE DESTINATION lib
	LIBRARY DESTINATION lib
)
