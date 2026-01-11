#include "rustine-dxc.hpp"

#include <dxc/dxcapi.h>
#include <string>
#include <vector>
#include <cstring>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#include <limits.h>
#include <unistd.h>
#include <cstdlib>
#include <string>
#include <filesystem>
namespace fs = std::filesystem;
#endif

// Internal compiler state
struct rdxc_compiler {
    IDxcUtils* utils;
    IDxcCompiler3* compiler;
    IDxcIncludeHandler* include_handler;
    
#ifdef _WIN32
    HMODULE library_handle;
#else
    void* library_handle;
#endif
};

// Helper to convert shader stage to HLSL profile
static const wchar_t* get_shader_profile(rdxc_shader_stage stage) {
    switch (stage) {
        case RDXC_SHADER_STAGE_VERTEX:   return L"vs_6_0";
        case RDXC_SHADER_STAGE_FRAGMENT: return L"ps_6_0";
        case RDXC_SHADER_STAGE_COMPUTE:  return L"cs_6_0";
        default: return L"";
    }
}

rdxc_status rdxcCompilerCreate(rdxc_compiler** out_compiler) {
    if (!out_compiler) {
        return RDXC_STATUS_INVALID_ARGUMENT;
    }

    rdxc_compiler* compiler = new rdxc_compiler{};
    
#ifdef _WIN32
    // Load dxcompiler.dll from same directory as executable
    compiler->library_handle = LoadLibraryW(L"dxcompiler.dll");
    if (!compiler->library_handle) {
        delete compiler;
        return RDXC_STATUS_LIBRARY_ERROR;
    }
    
    DxcCreateInstanceProc create_instance = 
        (DxcCreateInstanceProc)GetProcAddress(compiler->library_handle, "DxcCreateInstance");
#else
    // Try multiple strategies to locate libdxcompiler.so on Linux
    auto try_open_dxc = []() -> void* {
        // 1) Direct name (resolves via LD_LIBRARY_PATH / RPATH)
        void* handle = dlopen("libdxcompiler.so", RTLD_LAZY);
        if (handle) return handle;

        // 2) Respect DXC_LIB_DIR environment variable
        if (const char* env_dir = std::getenv("DXC_LIB_DIR")) {
            std::string candidate = std::string(env_dir) + "/libdxcompiler.so";
            handle = dlopen(candidate.c_str(), RTLD_LAZY);
            if (handle) return handle;
        }

        // 3) Same directory as the executable (/proc/self/exe)
        char exe_path[PATH_MAX] = {0};
        ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
        if (len > 0) {
            exe_path[len] = '\0';
            try {
                fs::path exe(exe_path);
                fs::path candidate = exe.parent_path() / "libdxcompiler.so";
                handle = dlopen(candidate.c_str(), RTLD_LAZY);
                if (handle) return handle;
            } catch (...) {
                // ignore filesystem errors and continue
            }
        }

        return nullptr;
    };

    compiler->library_handle = try_open_dxc();
    if (!compiler->library_handle) {
        // Surface a helpful diagnostic in logs for easier debugging under tools like RenderDoc
        const char* err = dlerror();
        (void)err; // suppress unused warning in release
        delete compiler;
        return RDXC_STATUS_LIBRARY_ERROR;
    }

    DxcCreateInstanceProc create_instance =
        (DxcCreateInstanceProc)dlsym(compiler->library_handle, "DxcCreateInstance");
#endif

    if (!create_instance) {
        rdxcCompilerDestroy(compiler);
        return RDXC_STATUS_LIBRARY_ERROR;
    }

    HRESULT hr = create_instance(CLSID_DxcUtils, IID_PPV_ARGS(&compiler->utils));
    if (FAILED(hr)) {
        rdxcCompilerDestroy(compiler);
        return RDXC_STATUS_LIBRARY_ERROR;
    }

    hr = create_instance(CLSID_DxcCompiler, IID_PPV_ARGS(&compiler->compiler));
    if (FAILED(hr)) {
        rdxcCompilerDestroy(compiler);
        return RDXC_STATUS_LIBRARY_ERROR;
    }

    hr = compiler->utils->CreateDefaultIncludeHandler(&compiler->include_handler);
    if (FAILED(hr)) {
        rdxcCompilerDestroy(compiler);
        return RDXC_STATUS_LIBRARY_ERROR;
    }

    *out_compiler = compiler;
    return RDXC_STATUS_OK;
}

rdxc_status rdxcCompileToSpirv(
    rdxc_compiler* compiler,
    const char* source,
    size_t source_length,
    const char* entry_point,
    rdxc_shader_stage stage,
    const char* const* defines,
    size_t define_count,
    rdxc_shader_result* out_result
) {
    if (!compiler || !source || !entry_point || !out_result) {
        return RDXC_STATUS_INVALID_ARGUMENT;
    }

    // Initialize result
    out_result->bytecode = nullptr;
    out_result->bytecode_size = 0;
    out_result->error_message = nullptr;

    // Create source blob
    IDxcBlobEncoding* source_blob = nullptr;
    HRESULT hr = compiler->utils->CreateBlob(
        source, 
        static_cast<uint32_t>(source_length), 
        CP_UTF8, 
        &source_blob
    );
    
    if (FAILED(hr)) {
        return RDXC_STATUS_LIBRARY_ERROR;
    }

    // Convert entry point to wide string
    std::wstring entry_point_wide(entry_point, entry_point + strlen(entry_point));

    // Build arguments
    std::vector<const wchar_t*> arguments;
    arguments.push_back(L"-spirv");              // Generate SPIR-V
    arguments.push_back(L"-fspv-target-env=vulkan1.3");
    arguments.push_back(L"-E");
    arguments.push_back(entry_point_wide.c_str());
    arguments.push_back(L"-T");
    arguments.push_back(get_shader_profile(stage));

    // Add defines
    std::vector<std::wstring> define_strings;
    for (size_t i = 0; i < define_count; ++i) {
        std::string define_str = std::string("-D") + defines[i];
        define_strings.emplace_back(define_str.begin(), define_str.end());
        arguments.push_back(define_strings.back().c_str());
    }

    // Compile
    DxcBuffer source_buffer{};
    source_buffer.Ptr = source_blob->GetBufferPointer();
    source_buffer.Size = source_blob->GetBufferSize();
    source_buffer.Encoding = CP_UTF8;

    IDxcResult* result = nullptr;
    hr = compiler->compiler->Compile(
        &source_buffer,
        arguments.data(),
        static_cast<uint32_t>(arguments.size()),
        compiler->include_handler,
        IID_PPV_ARGS(&result)
    );

    source_blob->Release();

    if (FAILED(hr)) {
        return RDXC_STATUS_LIBRARY_ERROR;
    }

    // Check compilation status
    HRESULT status_hr;
    result->GetStatus(&status_hr);

    if (FAILED(status_hr)) {
        // Get error messages
        IDxcBlobUtf8* errors = nullptr;
        result->GetOutput(DXC_OUT_ERRORS, IID_PPV_ARGS(&errors), nullptr);
        
        if (errors && errors->GetStringLength() > 0) {
            const char* error_str = errors->GetStringPointer();
            size_t error_len = errors->GetStringLength();
            out_result->error_message = new char[error_len + 1];
            memcpy(out_result->error_message, error_str, error_len);
            out_result->error_message[error_len] = '\0';
            errors->Release();
        }
        
        result->Release();
        return RDXC_STATUS_COMPILATION_FAILED;
    }

    // Get compiled bytecode
    IDxcBlob* bytecode_blob = nullptr;
    result->GetOutput(DXC_OUT_OBJECT, IID_PPV_ARGS(&bytecode_blob), nullptr);

    if (bytecode_blob) {
        out_result->bytecode_size = bytecode_blob->GetBufferSize();
        out_result->bytecode = new uint8_t[out_result->bytecode_size];
        memcpy(out_result->bytecode, bytecode_blob->GetBufferPointer(), out_result->bytecode_size);
        bytecode_blob->Release();
    }

    result->Release();
    return RDXC_STATUS_OK;
}

void rdxcShaderResultDestroy(rdxc_shader_result* result) {
    if (!result) {
        return;
    }

    if (result->bytecode) {
        delete[] result->bytecode;
        result->bytecode = nullptr;
    }

    if (result->error_message) {
        delete[] result->error_message;
        result->error_message = nullptr;
    }

    result->bytecode_size = 0;
}

void rdxcCompilerDestroy(rdxc_compiler* compiler) {
    if (!compiler) {
        return;
    }

    if (compiler->include_handler) {
        compiler->include_handler->Release();
    }

    if (compiler->compiler) {
        compiler->compiler->Release();
    }

    if (compiler->utils) {
        compiler->utils->Release();
    }

#ifdef _WIN32
    if (compiler->library_handle) {
        FreeLibrary(compiler->library_handle);
    }
#else
    if (compiler->library_handle) {
        dlclose(compiler->library_handle);
    }
#endif

    delete compiler;
}
