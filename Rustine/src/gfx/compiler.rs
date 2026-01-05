use std::ffi::{CStr, CString};
use std::ptr;

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

#[derive(Debug)]
pub struct ShaderCompileResult {
    pub bytecode: Vec<u8>,
    pub error_message: Option<String>,
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
    pub fn compile(
        &self,
        source: &str,
        entry_point: &str,
        stage: Stage,
        defines: &[&str],
    ) -> Result<ShaderCompileResult, Status> {
        let entry_point_c = CString::new(entry_point).map_err(|_| Status::InvalidArgument)?;

        let define_cstrings: Vec<CString> = defines
            .iter()
            .map(|s| CString::new(*s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Status::InvalidArgument)?;

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
            return Err(status);
        }

        Ok(ShaderCompileResult {
            bytecode,
            error_message,
        })
    }
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
