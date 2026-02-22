#![allow(dead_code)]

use std::error::Error;
use std::ffi::{CStr, CString};
use std::fs;
use std::path::Path;
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

/// Build-time shader compilation: reads shaders and config, generates Rust code.
/// Writes shader.rs to out_dir and returns the generated module code.
/// Suitable for use in build.rs scripts.
pub fn build_shaders(
    shaders_dir: &Path,
    config_path: &Path,
    out_dir: &Path,
) -> Result<String, Box<dyn Error>> {
    // Collect .hlsl files
    let mut shader_files: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(shaders_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && ext.to_string_lossy().to_lowercase() == "hlsl"
            {
                shader_files.push(path);
            }
        }
    }

    // Sort for deterministic ordering
    shader_files.sort();

    // If there's no shaders.toml, nothing to do
    if !config_path.exists() {
        return Ok(String::new());
    }

    let config_content = fs::read_to_string(&config_path)?;
    let config = toml::from_str::<toml::Value>(&config_content)?;

    let mut module_code = String::new();
    module_code.push_str("// Auto-generated shader modules\n\n");
    module_code.push_str("// Generated bytecode blobs and accessor\n\n");

    // Generate shader IDs starting from u32::MAX - 2
    let mut shader_id = u32::MAX - 2u32;
    let mut shader_info: Vec<(String, u32, Vec<String>)> = Vec::new();

    // Create a compiler instance
    let compiler = Compiler::new()
        .map_err(|e| format!("Failed to create shader compiler: {:?}", e))?;

    if let Some(shaders_table) = config.get("shaders").and_then(|v| v.as_table()) {
        for (name, shader_config) in shaders_table {
            let mut stages_vec: Vec<String> = Vec::new();
            if let Some(table) = shader_config.as_table()
                && let Some(stages) = table.get("stages").and_then(|v| v.as_array())
            {
                for s in stages {
                    if let Some(s_str) = s.as_str() {
                        stages_vec.push(s_str.to_string());
                    }
                }
            }

            // Read shader source
            let shader_path = shaders_dir.join(format!("{}.hlsl", name));
            let source = fs::read_to_string(&shader_path)?;

            // Compile each stage
            for stage_name in &stages_vec {
                let stage_enum = match stage_name.as_str() {
                    "vertex" => Stage::VERTEX,
                    "fragment" => Stage::FRAGMENT,
                    "compute" => Stage::COMPUTE,
                    other => {
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("Unknown shader stage '{}' for shader '{}'", other, name),
                        )) as Box<dyn Error>);
                    }
                };

                let compiled = compiler.compile(stage_enum, &source).map_err(|e| {
                    let msg = match e {
                        CompilerStatus::CompilationFailed(msg) => {
                            format!("Shader compilation failed for {}:{} => {}", name, stage_name, msg)
                        }
                        _ => format!("Shader compilation error for {}:{}", name, stage_name),
                    };
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)) as Box<dyn Error>
                })?;

                // Emit a const byte array
                let const_name = format!("{}_{}", name.to_uppercase(), stage_name.to_uppercase());
                module_code.push_str(&format!("pub static {}: &[u8] = &[\n", const_name));
                if compiled.bytecode.is_empty() {
                    module_code.push_str("];\n");
                } else {
                    // write in hex for readability
                    for chunk in compiled.bytecode.chunks(16) {
                        module_code.push_str("    ");
                        for b in chunk {
                            module_code.push_str(&format!("0x{:02x}, ", b));
                        }
                        module_code.push('\n');
                    }
                    module_code.push_str("];\n");
                }
                module_code.push('\n');
            }

            // store info for registry generation
            shader_info.push((name.clone(), shader_id, stages_vec));
            shader_id -= 1;
        }
    }

    // Generate constants and get_shaderprogram function
    for (name, id, _stages) in &shader_info {
        let const_name = format!("{}_SHADER_ID", name.to_uppercase());
        module_code.push_str(&format!(
            "pub const {}: u32 = {};// shader id\n",
            const_name, id
        ));
    }

    module_code.push_str("\nuse std::vec::Vec;\n\n");
    module_code.push_str("pub fn get_shaderprogram(id: u32) -> Option<::rustinesc::ShaderProgram> {\n");
    module_code.push_str("    match id {\n");
    for (name, id, stages) in &shader_info {
        module_code.push_str(&format!("        {} => {{\n", id));
        module_code.push_str("            let mut stages = Vec::new();\n");
        for stage_name in stages {
            let const_name = format!("{}_{}", name.to_uppercase(), stage_name.to_uppercase());
            let stage_enum = match stage_name.as_str() {
                "vertex" => "rustinesc::Stage::VERTEX",
                "fragment" => "rustinesc::Stage::FRAGMENT",
                "compute" => "rustinesc::Stage::COMPUTE",
                _ => unreachable!(),
            };
            module_code.push_str(&format!(
                "            if !{}.is_empty() {{\n                stages.push(rustinesc::Shader {{ stage: {}, entry_point: \"{}\".to_string(), bytecode: {}.to_vec() }});\n            }}\n",
                const_name, stage_enum, stage_name, const_name
            ));
        }
        module_code.push_str(&format!(
            "            Some(rustinesc::ShaderProgram {{ name: \"{}\".to_string(), stages }})\n",
            name
        ));
        module_code.push_str("        },\n");
    }
    module_code.push_str("        _ => None,\n");
    module_code.push_str("    }\n");
    module_code.push_str("}\n");

    let output_path = out_dir.join("shaders.rs");
    fs::write(&output_path, &module_code)?;

    Ok(module_code)
}

/// Build-time wrapper for build_shaders that emits cargo directives.
/// Use this in build.rs scripts.
pub fn build_shaders_emit_directives(
    shaders_dir: &Path,
    config_path: &Path,
    out_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let _module_code = build_shaders(shaders_dir, config_path, out_dir)?;

    println!("cargo:rerun-if-changed={}", config_path.display());
    if let Ok(entries) = fs::read_dir(shaders_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "hlsl" {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }

    Ok(())
}
