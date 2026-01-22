pub type Vector2i = Vector2<i32>;
pub type Vector2u = Vector2<u32>;
pub type Vector2f = Vector2<f32>;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vector2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl Default for Vector2<u32> {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl Default for Vector2<f32> {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl std::ops::Add for Vector2<f32> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Vector2<f32> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for Vector2<f32> {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl std::ops::Mul<Vector2<f32>> for f32 {
    type Output = Vector2<f32>;
    fn mul(self, vec: Vector2<f32>) -> Vector2<f32> {
        Vector2 {
            x: self * vec.x,
            y: self * vec.y,
        }
    }
}

impl std::ops::Neg for Vector2<f32> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Vector2<f32> {
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self { x: 0.0, y: 0.0 }
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Computes the Euclidean distance between this vector and another.
    pub fn distance(&self, other: &Self) -> f32 {
        (*other - *self).length()
    }
}

pub type Vector3f = Vector3<f32>;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vector3<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl Default for Vector3<f32> {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl std::ops::Add for Vector3<f32> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Sub for Vector3<f32> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Mul<f32> for Vector3<f32> {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl std::ops::Mul<Vector3<f32>> for f32 {
    type Output = Vector3<f32>;
    fn mul(self, vec: Vector3<f32>) -> Vector3<f32> {
        Vector3 {
            x: self * vec.x,
            y: self * vec.y,
            z: self * vec.z,
        }
    }
}

impl std::ops::Neg for Vector3<f32> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Vector3<f32> {
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product (right-handed)
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Euclidean distance between this vector and another.
    pub fn distance(&self, other: &Self) -> f32 {
        (*other - *self).length()
    }
}

pub type Vector4f = Vector4<f32>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vector4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

impl Default for Vector4<f32> {
    fn default() -> Self {
        Vector4::<f32>::new(0.0, 0.0, 0.0, 0.0)
    }
}

impl Vector4<f32> {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl From<Color> for Vector4<f32> {
    fn from(color: Color) -> Self {
        Self {
            x: color.r,
            y: color.g,
            z: color.b,
            w: color.a,
        }
    }
}

/// Color with red, green, blue, and alpha components.
///
/// Each component is a floating-point value typically in the range [0.0, 1.0].
///
/// Layout-compatible with `Vector4<f32>`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<Vector3<f32>> for Color {
    /// Creates a `Color` from a `Vector3`, setting alpha to 1.0.
    fn from(vec: Vector3<f32>) -> Self {
        Self {
            r: vec.x,
            g: vec.y,
            b: vec.z,
            a: 1.0,
        }
    }
}

impl From<Vector4<f32>> for Color {
    fn from(vec: Vector4<f32>) -> Self {
        Self {
            r: vec.x,
            g: vec.y,
            b: vec.z,
            a: vec.w,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::new(0.0, 0.0, 0.0, 0.0)
    }
}

impl Color {
    /// Creates a new `Color` with the specified red, green, blue, and alpha components.
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Returns a new `Color` with the specified alpha value.
    pub fn with_alpha(&self, alpha: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }

    /// Creates a `Color` from a 32-bit unsigned integer in RGBA format.
    pub fn from_u32(rgba: u32) -> Self {
        let r = ((rgba >> 24) & 0xFF) as f32 / 255.0;
        let g = ((rgba >> 16) & 0xFF) as f32 / 255.0;
        let b = ((rgba >> 8) & 0xFF) as f32 / 255.0;
        let a = (rgba & 0xFF) as f32 / 255.0;
        Self { r, g, b, a }
    }

    /// Packs the color into a 32-bit unsigned integer in RGBA format.
    pub fn to_u32(&self) -> u32 {
        ((self.r.clamp(0.0, 1.0) * 255.0) as u32) << 24
            | ((self.g.clamp(0.0, 1.0) * 255.0) as u32) << 16
            | ((self.b.clamp(0.0, 1.0) * 255.0) as u32) << 8
            | ((self.a.clamp(0.0, 1.0) * 255.0) as u32)
    }
}
