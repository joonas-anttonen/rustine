cmake_minimum_required(VERSION 3.16)
project(rustine_dxc_interop)

# Allow overriding the library name (default: rustine-dxc)
set(RUSTINE_DXC_TARGET "rustine-dxc" CACHE STRING "Library target name")

# Create the interop library
add_library(${RUSTINE_DXC_TARGET} STATIC
    rustine-dxc.cpp
)

# Use system-provided DXC headers from nix environment
# The DXC_INCLUDE_DIR can be set via environment or CMake variable
if(DEFINED ENV{DXC_INCLUDE_DIR})
    target_include_directories(${RUSTINE_DXC_TARGET} PRIVATE $ENV{DXC_INCLUDE_DIR})
else()
    # Fallback: assume system headers are in standard location
    target_include_directories(${RUSTINE_DXC_TARGET} PRIVATE "/usr/include")
endif()

# Set C++ standard
set_property(TARGET ${RUSTINE_DXC_TARGET} PROPERTY CXX_STANDARD 17)
set_property(TARGET ${RUSTINE_DXC_TARGET} PROPERTY CXX_STANDARD_REQUIRED ON)

# Set output directories
set_target_properties(${RUSTINE_DXC_TARGET} PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
    LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib
    ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib
)

# Install
install(TARGETS ${RUSTINE_DXC_TARGET}
    RUNTIME DESTINATION bin
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)
