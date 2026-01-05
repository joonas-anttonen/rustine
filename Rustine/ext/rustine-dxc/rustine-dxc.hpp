#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Status codes for DXC operations
typedef enum rdxc_status {
    RDXC_STATUS_OK = 0,
    RDXC_STATUS_INVALID_ARGUMENT = 1,
    RDXC_STATUS_COMPILATION_FAILED = 2,
    RDXC_STATUS_LIBRARY_ERROR = 3,
} rdxc_status;

// Shader stage types
typedef enum rdxc_shader_stage {
    RDXC_SHADER_STAGE_VERTEX,
    RDXC_SHADER_STAGE_FRAGMENT,
    RDXC_SHADER_STAGE_COMPUTE,
} rdxc_shader_stage;

// Opaque compiler instance
typedef struct rdxc_compiler rdxc_compiler;

// Compiled shader result
typedef struct rdxc_shader_result {
    uint8_t* bytecode;
    size_t bytecode_size;
    char* error_message;
} rdxc_shader_result;

// Initialize the DXC compiler
rdxc_status rdxcCompilerCreate(rdxc_compiler** out_compiler);

// Compile HLSL source code to SPIR-V
rdxc_status rdxcCompileToSpirv(
    rdxc_compiler* compiler,
    const char* source,
    size_t source_length,
    const char* entry_point,
    rdxc_shader_stage stage,
    const char* const* defines,
    size_t define_count,
    rdxc_shader_result* out_result
);

// Free shader result resources
void rdxcShaderResultDestroy(rdxc_shader_result* result);

// Destroy the compiler instance
void rdxcCompilerDestroy(rdxc_compiler* compiler);

#ifdef __cplusplus
}
#endif
