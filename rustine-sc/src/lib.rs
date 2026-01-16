#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::ptr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerStatus {
    Ok,
    Error,
    CompilationFailed(String),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdxStatus {
    Ok = 0,
    InvalidArgument = 1,
    CompilationFailed = 2,
    LibraryError = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage(pub u32);
impl std::ops::BitAnd for Stage {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Stage(self.0 & rhs.0)
    }
}
impl std::ops::BitOr for Stage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Stage(self.0 | rhs.0)
    }
}
impl Stage {
    pub const VERTEX: Stage = Stage(VkShaderStageFlags::VERTEX_BIT.0);
    pub const FRAGMENT: Stage = Stage(VkShaderStageFlags::FRAGMENT_BIT.0);
    pub const COMPUTE: Stage = Stage(VkShaderStageFlags::COMPUTE_BIT.0);
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
    fn rdxcCompilerCreate(out_compiler: *mut *mut rdxc_compiler) -> RdxStatus;

    fn rdxcCompileToSpirv(
        compiler: *mut rdxc_compiler,
        source: *const u8,
        source_length: usize,
        entry_point: *const i8,
        stage: Stage,
        defines: *const *const i8,
        define_count: usize,
        out_result: *mut RdxcShaderResult,
    ) -> RdxStatus;

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
    pub fn new() -> Result<Self, CompilerStatus> {
        let mut handle = ptr::null_mut();
        let status = unsafe { rdxcCompilerCreate(&mut handle) };

        if status != RdxStatus::Ok {
            return Err(CompilerStatus::Error);
        }

        Ok(Self { handle })
    }

    /// Compiles the given shader source code to SPIR-V bytecode.
    pub fn compile(&self, stage: Stage, source: &str) -> Result<Shader, CompilerStatus> {
        let entry_point = match stage {
            Stage::VERTEX => Ok("vertex"),
            Stage::FRAGMENT => Ok("fragment"),
            Stage::COMPUTE => Ok("compute"),
            _ => Err(CompilerStatus::Error),
        }?;
        let entry_point_c = CString::new(entry_point).map_err(|_| CompilerStatus::Error)?;

        let define_cstrings: Vec<CString> = ["RUSTINE"]
            .iter()
            .map(|s| CString::new(*s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| CompilerStatus::Error)?;

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
                // Copy the bytecode from the raw pointer
                std::slice::from_raw_parts(result.bytecode, result.bytecode_size).to_vec()
            }
        } else {
            Vec::new()
        };

        let error_message = if !result.error_message.is_null() {
            unsafe {
                Some(
                    CStr::from_ptr(result.error_message)
                        .to_string_lossy()
                        .into_owned()
                        .trim()
                        .to_string(),
                )
            }
        } else {
            None
        };

        unsafe {
            rdxcShaderResultDestroy(&mut result);
        }

        if status != RdxStatus::Ok {
            if status == RdxStatus::CompilationFailed {
                return Err(CompilerStatus::CompilationFailed(
                    error_message.unwrap_or_else(|| "Unknown error".to_string()),
                ));
            } else {
                return Err(CompilerStatus::Error);
            }
        }

        Ok(Shader {
            stage,
            entry_point: entry_point.to_string(),
            bytecode,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VkShaderStageFlags(pub u32);
impl VkShaderStageFlags {
    pub const VERTEX_BIT: Self = Self(0x00000001);
    pub const TESSELLATION_CONTROL_BIT: Self = Self(0x00000002);
    pub const TESSELLATION_EVALUATION_BIT: Self = Self(0x00000004);
    pub const GEOMETRY_BIT: Self = Self(0x00000008);
    pub const FRAGMENT_BIT: Self = Self(0x00000010);
    pub const COMPUTE_BIT: Self = Self(0x00000020);
    pub const ALL_GRAPHICS: Self = Self(0x0000001F);
    pub const ALL: Self = Self(0x7FFFFFFF);
    pub const RAYGEN_BIT_KHR: Self = Self(0x00000100);
    pub const ANY_HIT_BIT_KHR: Self = Self(0x00000200);
    pub const CLOSEST_HIT_BIT_KHR: Self = Self(0x00000400);
    pub const MISS_BIT_KHR: Self = Self(0x00000800);
    pub const INTERSECTION_BIT_KHR: Self = Self(0x00001000);
    pub const CALLABLE_BIT_KHR: Self = Self(0x00002000);
    pub const TASK_BIT_EXT: Self = Self(0x00000040);
    pub const MESH_BIT_EXT: Self = Self(0x00000080);
    pub const SUBPASS_SHADING_BIT_HUAWEI: Self = Self(0x00004000);
    pub const CLUSTER_CULLING_BIT_HUAWEI: Self = Self(0x00080000);
    pub const RAYGEN_BIT_NV: Self = Self(0x00000100);
    pub const ANY_HIT_BIT_NV: Self = Self(0x00000200);
    pub const CLOSEST_HIT_BIT_NV: Self = Self(0x00000400);
    pub const MISS_BIT_NV: Self = Self(0x00000800);
    pub const INTERSECTION_BIT_NV: Self = Self(0x00001000);
    pub const CALLABLE_BIT_NV: Self = Self(0x00002000);
    pub const TASK_BIT_NV: Self = Self(0x00000040);
    pub const MESH_BIT_NV: Self = Self(0x00000080);
}
