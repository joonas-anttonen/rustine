pub mod parameters;
pub use parameters::ApiParameters;
pub mod vulkan;
pub mod core;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Decodes a Vulkan API version integer into a Version struct.
    /// 
    /// Vulkan encodes versions as: bits 31-22: major, bits 21-12: minor, bits 11-0: patch.
    pub fn from_vk_version(vk_version: u32) -> Self {
        let major = (vk_version >> 22) & 0x3FF;
        let minor = (vk_version >> 12) & 0x3FF;
        let patch = vk_version & 0xFFF;
        Self { major, minor, patch }
    }

    /// Encodes a Version into a Vulkan API version integer.
    /// 
    /// Returns the packed Vulkan format: bits 31-22: major, bits 21-12: minor, bits 11-0: patch.
    pub fn to_vk_version(&self) -> u32 {
        (self.major << 22) | (self.minor << 12) | self.patch
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}