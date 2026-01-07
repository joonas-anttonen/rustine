use std::ffi::{CStr, CString};
use std::ptr;

use crate::gfx::vulkan as vk;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    InvalidArgument = 1,
    CompilationFailed = 2,
    LibraryError = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Vertex,
    Fragment,
    Compute,
}

impl Stage {
    pub fn to_vk(&self) -> vk::VkShaderStageFlags {
        match self {
            Stage::Vertex => vk::VkShaderStageFlags::VERTEX_BIT,
            Stage::Fragment => vk::VkShaderStageFlags::FRAGMENT_BIT,
            Stage::Compute => vk::VkShaderStageFlags::COMPUTE_BIT,
        }
    }
}

pub struct Shader {
    pub stage: Stage,
    pub entry_point: String,
    pub bytecode: Vec<u8>,
}

pub struct ShaderProgram {
    pub name: String,
    pub stages: Vec<Shader>,
}

impl ShaderProgram {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stages: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage: Shader) {
        if self.stages.iter().any(|s| s.stage == stage.stage) {
            debug_assert!(false, "Duplicate shader stage added");
            return;
        }

        self.stages.push(stage);
    }
}

#[repr(C)]
struct RdxcShaderResult {
    bytecode: *mut u8,
    bytecode_size: usize,
    error_message: *mut i8,
}

#[allow(non_camel_case_types)]
type rdxc_compiler = std::ffi::c_void;

#[link(name = "rustine-dxc", kind = "static")]
unsafe extern "C" {
    fn rdxcCompilerCreate(out_compiler: *mut *mut rdxc_compiler) -> Status;

    fn rdxcCompileToSpirv(
        compiler: *mut rdxc_compiler,
        source: *const u8,
        source_length: usize,
        entry_point: *const i8,
        stage: Stage,
        defines: *const *const i8,
        define_count: usize,
        out_result: *mut RdxcShaderResult,
    ) -> Status;

    fn rdxcShaderResultDestroy(result: *mut RdxcShaderResult);
    fn rdxcCompilerDestroy(compiler: *mut rdxc_compiler);
}

pub struct Compiler {
    handle: *mut rdxc_compiler,
}

impl Drop for Compiler {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                rdxcCompilerDestroy(self.handle);
            }
        }
    }
}

impl Compiler {
    /// Creates a new instance of the DXC compiler.
    pub fn new() -> Result<Self, Status> {
        let mut handle = ptr::null_mut();
        let status = unsafe { rdxcCompilerCreate(&mut handle) };

        if status != Status::Ok {
            return Err(status);
        }

        Ok(Self { handle })
    }

    /// Compiles the given shader source code to SPIR-V bytecode.
    pub fn compile(&self, stage: Stage, source: &str) -> Result<Shader, String> {
        let entry_point = match stage {
            Stage::Vertex => "vertex",
            Stage::Fragment => "fragment",
            Stage::Compute => "compute",
        };
        let entry_point_c = CString::new(entry_point).map_err(|_| "Invalid entry point string")?;

        let define_cstrings: Vec<CString> = ["RUSTINE"]
            .iter()
            .map(|s| CString::new(*s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Invalid define string")?;

        let define_ptrs: Vec<*const i8> = define_cstrings.iter().map(|s| s.as_ptr()).collect();

        let mut result = RdxcShaderResult {
            bytecode: ptr::null_mut(),
            bytecode_size: 0,
            error_message: ptr::null_mut(),
        };

        let status = unsafe {
            rdxcCompileToSpirv(
                self.handle,
                source.as_ptr(),
                source.len(),
                entry_point_c.as_ptr(),
                stage,
                if define_ptrs.is_empty() {
                    ptr::null()
                } else {
                    define_ptrs.as_ptr()
                },
                define_ptrs.len(),
                &mut result,
            )
        };

        let bytecode = if !result.bytecode.is_null() && result.bytecode_size > 0 {
            unsafe {
                Vec::from_raw_parts(result.bytecode, result.bytecode_size, result.bytecode_size)
            }
        } else {
            Vec::new()
        };

        let error_message = if !result.error_message.is_null() {
            unsafe {
                let c_str = CStr::from_ptr(result.error_message);
                let message = c_str.to_string_lossy().into_owned();
                libc::free(result.error_message as *mut libc::c_void);
                Some(message)
            }
        } else {
            None
        };

        if status != Status::Ok {
            return Err(error_message.unwrap_or_else(|| "Unknown error".to_string()));
        }

        Ok(Shader {
            stage,
            entry_point: entry_point.to_string(),
            bytecode,
        })
    }
}
