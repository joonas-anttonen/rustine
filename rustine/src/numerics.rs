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
    pub fn norm_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn norm(&self) -> f32 {
        self.norm_squared().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.norm();
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
        (*other - *self).norm()
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
    pub fn norm_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn norm(&self) -> f32 {
        self.norm_squared().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.norm();
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
        (*other - *self).norm()
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

pub type Quaternionf = Quaternion<f32>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Quaternion<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

impl Quaternion<f32> {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn from_rotation(rotation: &Matrix3f) -> Self {
        // Assumes `rotation` is a pure rotation matrix (no scale).
        // Elements: matrix is column-major stored as c0,c1,c2 where
        // c0 = (m00, m10, m20), c1 = (m01, m11, m21), c2 = (m02, m12, m22)
        let m00 = rotation.c0.x;
        let m01 = rotation.c1.x;
        let m02 = rotation.c2.x;

        let m10 = rotation.c0.y;
        let m11 = rotation.c1.y;
        let m12 = rotation.c2.y;

        let m20 = rotation.c0.z;
        let m21 = rotation.c1.z;
        let m22 = rotation.c2.z;

        let trace = m00 + m11 + m22;

        let (mut qx, mut qy, mut qz, mut qw);

        if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            qw = 0.25 * s;
            qx = (m21 - m12) / s;
            qy = (m02 - m20) / s;
            qz = (m10 - m01) / s;
        } else if m00 > m11 && m00 > m22 {
            let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
            qw = (m21 - m12) / s;
            qx = 0.25 * s;
            qy = (m01 + m10) / s;
            qz = (m02 + m20) / s;
        } else if m11 > m22 {
            let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
            qw = (m02 - m20) / s;
            qx = (m01 + m10) / s;
            qy = 0.25 * s;
            qz = (m12 + m21) / s;
        } else {
            let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
            qw = (m10 - m01) / s;
            qx = (m02 + m20) / s;
            qy = (m12 + m21) / s;
            qz = 0.25 * s;
        }

        // Normalize to protect against numerical drift
        let len = (qx * qx + qy * qy + qz * qz + qw * qw).sqrt();
        if len > 1e-8 {
            qx /= len;
            qy /= len;
            qz /= len;
            qw /= len;
        } else {
            return Self::identity();
        }

        Self::new(qx, qy, qz, qw)
    }

    pub fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

pub type Matrix4f = Matrix4<f32>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Matrix4<T> {
    pub c0: Vector4<T>,
    pub c1: Vector4<T>,
    pub c2: Vector4<T>,
    pub c3: Vector4<T>,
}

impl Matrix4<f32> {
    pub fn new(
        m00: f32,
        m01: f32,
        m02: f32,
        m03: f32,
        m10: f32,
        m11: f32,
        m12: f32,
        m13: f32,
        m20: f32,
        m21: f32,
        m22: f32,
        m23: f32,
        m30: f32,
        m31: f32,
        m32: f32,
        m33: f32,
    ) -> Self {
        Self {
            c0: Vector4::new(m00, m10, m20, m30),
            c1: Vector4::new(m01, m11, m21, m31),
            c2: Vector4::new(m02, m12, m22, m32),
            c3: Vector4::new(m03, m13, m23, m33),
        }
    }

    /// Creates a `Matrix4f` from a slice of 16 `f32` values.
    ///
    /// The slice must have at least 16 elements and is expected to be in column-major order.
    pub fn from_slice(slice: &[f32]) -> Self {
        Self {
            c0: Vector4::new(slice[0], slice[1], slice[2], slice[3]),
            c1: Vector4::new(slice[4], slice[5], slice[6], slice[7]),
            c2: Vector4::new(slice[8], slice[9], slice[10], slice[11]),
            c3: Vector4::new(slice[12], slice[13], slice[14], slice[15]),
        }
    }

    pub fn from_trs(t: Vector3f, r: Quaternionf, s: Vector3f) -> Self {
        let x = r.x;
        let y = r.y;
        let z = r.z;
        let w = r.w;

        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;

        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;

        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;

        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;

        // Rotation matrix (row-major elements)
        let m00 = 1.0 - (yy + zz);
        let m01 = xy + wz;
        let m02 = xz - wy;

        let m10 = xy - wz;
        let m11 = 1.0 - (xx + zz);
        let m12 = yz + wx;

        let m20 = xz + wy;
        let m21 = yz - wx;
        let m22 = 1.0 - (xx + yy);

        // Column-major storage: scale applied per-column
        let c0 = Vector4::new(m00 * s.x, m10 * s.x, m20 * s.x, 0.0);
        let c1 = Vector4::new(m01 * s.y, m11 * s.y, m21 * s.y, 0.0);
        let c2 = Vector4::new(m02 * s.z, m12 * s.z, m22 * s.z, 0.0);
        let c3 = Vector4::new(t.x, t.y, t.z, 1.0);

        Self { c0, c1, c2, c3 }
    }

    pub fn identity() -> Self {
        Self {
            c0: Vector4::new(1.0, 0.0, 0.0, 0.0),
            c1: Vector4::new(0.0, 1.0, 0.0, 0.0),
            c2: Vector4::new(0.0, 0.0, 1.0, 0.0),
            c3: Vector4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Extracts the linear (rotation and scale) part of the matrix as a `Matrix3f`.
    pub fn linear(&self) -> Matrix3f {
        Matrix3f {
            c0: Vector3f::new(self.c0.x, self.c0.y, self.c0.z),
            c1: Vector3f::new(self.c1.x, self.c1.y, self.c1.z),
            c2: Vector3f::new(self.c2.x, self.c2.y, self.c2.z),
        }
    }

    pub fn rotation(&self) -> Matrix3f {
        // Extract 3x3 linear part
        let m3 = self.linear();

        // column lengths = scale per-axis
        let sx = (m3.c0.x * m3.c0.x + m3.c0.y * m3.c0.y + m3.c0.z * m3.c0.z).sqrt();
        let sy = (m3.c1.x * m3.c1.x + m3.c1.y * m3.c1.y + m3.c1.z * m3.c1.z).sqrt();
        let sz = (m3.c2.x * m3.c2.x + m3.c2.y * m3.c2.y + m3.c2.z * m3.c2.z).sqrt();

        // prevent division by zero
        let inv_sx = if sx > 1e-8 { 1.0 / sx } else { 1.0 };
        let inv_sy = if sy > 1e-8 { 1.0 / sy } else { 1.0 };
        let inv_sz = if sz > 1e-8 { 1.0 / sz } else { 1.0 };

        // Build a rotation-only matrix by normalizing each column (remove scale)
        let c0 = Vector3f::new(m3.c0.x * inv_sx, m3.c0.y * inv_sx, m3.c0.z * inv_sx);
        let c1 = Vector3f::new(m3.c1.x * inv_sy, m3.c1.y * inv_sy, m3.c1.z * inv_sy);
        let c2 = Vector3f::new(m3.c2.x * inv_sz, m3.c2.y * inv_sz, m3.c2.z * inv_sz);

        Matrix3f { c0, c1, c2 }
    }

    /// Extracts the translation part of the matrix as a `Vector3f`.
    pub fn translation(&self) -> Vector3f {
        Vector3f::new(self.c3.x, self.c3.y, self.c3.z)
    }

    pub fn col(&self, index: usize) -> Vector4<f32> {
        match index {
            0 => self.c0,
            1 => self.c1,
            2 => self.c2,
            3 => self.c3,
            _ => panic!("Column index out of bounds"),
        }
    }

    pub fn row(&self, index: usize) -> Vector4<f32> {
        match index {
            0 => Vector4::new(self.c0.x, self.c1.x, self.c2.x, self.c3.x),
            1 => Vector4::new(self.c0.y, self.c1.y, self.c2.y, self.c3.y),
            2 => Vector4::new(self.c0.z, self.c1.z, self.c2.z, self.c3.z),
            3 => Vector4::new(self.c0.w, self.c1.w, self.c2.w, self.c3.w),
            _ => panic!("Row index out of bounds"),
        }
    }
}

impl std::fmt::Debug for Matrix4<f32> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Matrix4")
            .field("c0", &self.c0)
            .field("c1", &self.c1)
            .field("c2", &self.c2)
            .field("c3", &self.c3)
            .finish()
    }
}

pub type Matrix3f = Matrix3<f32>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Matrix3<T> {
    pub c0: Vector3<T>,
    pub c1: Vector3<T>,
    pub c2: Vector3<T>,
}

impl Matrix3<f32> {
    pub fn new(
        m00: f32,
        m01: f32,
        m02: f32,
        m10: f32,
        m11: f32,
        m12: f32,
        m20: f32,
        m21: f32,
        m22: f32,
    ) -> Self {
        Self {
            c0: Vector3::new(m00, m10, m20),
            c1: Vector3::new(m01, m11, m21),
            c2: Vector3::new(m02, m12, m22),
        }
    }

    pub fn column(&self, index: usize) -> Vector3<f32> {
        match index {
            0 => self.c0,
            1 => self.c1,
            2 => self.c2,
            _ => panic!("Column index out of bounds"),
        }
    }

    pub fn row(&self, index: usize) -> Vector3<f32> {
        match index {
            0 => Vector3::new(self.c0.x, self.c1.x, self.c2.x),
            1 => Vector3::new(self.c0.y, self.c1.y, self.c2.y),
            2 => Vector3::new(self.c0.z, self.c1.z, self.c2.z),
            _ => panic!("Row index out of bounds"),
        }
    }
}
