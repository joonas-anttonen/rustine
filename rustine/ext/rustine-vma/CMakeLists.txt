cmake_minimum_required(VERSION 3.16)
project(rustine_vma_interop)

# Allow overriding the library name (default: rustine-vma)
set(RUSTINE_VMA_TARGET "rustine-vma" CACHE STRING "Library target name")

# Create the interop library
add_library(${RUSTINE_VMA_TARGET} STATIC
    rustine-vma.cpp
)

# Include directories for the interop library
target_include_directories(${RUSTINE_VMA_TARGET} PRIVATE "../vma/include")

# Link with Vulkan
find_package(Vulkan REQUIRED)
target_link_libraries(${RUSTINE_VMA_TARGET} PRIVATE Vulkan::Vulkan)

# Link with VulkanMemoryAllocator
target_link_libraries(${RUSTINE_VMA_TARGET} PRIVATE VulkanMemoryAllocator)

# Set output directories
set_target_properties(${RUSTINE_VMA_TARGET} PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
    LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib
    ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib
)

# Install
install(TARGETS ${RUSTINE_VMA_TARGET}
    RUNTIME DESTINATION bin
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)