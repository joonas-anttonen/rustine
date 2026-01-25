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

    pub fn transform(&self, m: &Matrix4f) -> Self {
        // Multiply the vector (as a point with w = 1) by the matrix.
        // Column-major multiplication: result = M * (v.x, v.y, v.z, 1)
        let x = m.c0.x * self.x + m.c1.x * self.y + m.c2.x * self.z + m.c3.x;
        let y = m.c0.y * self.x + m.c1.y * self.y + m.c2.y * self.z + m.c3.y;
        let z = m.c0.z * self.x + m.c1.z * self.y + m.c2.z * self.z + m.c3.z;
        let w = m.c0.w * self.x + m.c1.w * self.y + m.c2.w * self.z + m.c3.w;

        // If w is not ~1, perform perspective divide. If w is (near) zero,
        // return the un-divided result (treat as direction).
        if w.abs() > 1e-6 {
            Self::new(x / w, y / w, z / w)
        } else {
            Self::new(x, y, z)
        }
    }

    /// Transforms a surface normal (direction-only vector) by the matrix.
    ///
    /// Normals must be transformed by the inverse-transpose of the linear
    /// part of the transform to remain correct under non-uniform scaling.
    pub fn transform_normal(&self, m: &Matrix4f) -> Self {
        let lin = m.linear();
        if let Some(inv) = lin.inverse() {
            let inv_t = inv.transpose();
            inv_t.mul_vec(*self)
        } else {
            // Fallback: if non-invertible, apply the linear part directly.
            lin.mul_vec(*self)
        }
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
        let m01 = xy - wz;
        let m02 = xz + wy;

        let m10 = xy + wz;
        let m11 = 1.0 - (xx + zz);
        let m12 = yz - wx;

        let m20 = xz - wy;
        let m21 = yz + wx;
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

    /// Returns the transpose of this matrix.
    pub fn transpose(&self) -> Self {
        let m00 = self.c0.x;
        let m01 = self.c1.x;
        let m02 = self.c2.x;

        let m10 = self.c0.y;
        let m11 = self.c1.y;
        let m12 = self.c2.y;

        let m20 = self.c0.z;
        let m21 = self.c1.z;
        let m22 = self.c2.z;

        // Construct transpose using Matrix3::new(m00, m01, m02, m10, m11, m12, m20, m21, m22)
        Matrix3::new(m00, m10, m20, m01, m11, m21, m02, m12, m22)
    }

    /// Attempts to invert the matrix. Returns `None` if the matrix is singular.
    pub fn inverse(&self) -> Option<Self> {
        let m00 = self.c0.x;
        let m01 = self.c1.x;
        let m02 = self.c2.x;

        let m10 = self.c0.y;
        let m11 = self.c1.y;
        let m12 = self.c2.y;

        let m20 = self.c0.z;
        let m21 = self.c1.z;
        let m22 = self.c2.z;

        let det = m00 * (m11 * m22 - m12 * m21)
            - m01 * (m10 * m22 - m12 * m20)
            + m02 * (m10 * m21 - m11 * m20);

        if det.abs() < 1e-12 {
            return None;
        }

        let inv_det = 1.0 / det;

        let i00 = (m11 * m22 - m12 * m21) * inv_det;
        let i01 = (m02 * m21 - m01 * m22) * inv_det;
        let i02 = (m01 * m12 - m02 * m11) * inv_det;

        let i10 = (m12 * m20 - m10 * m22) * inv_det;
        let i11 = (m00 * m22 - m02 * m20) * inv_det;
        let i12 = (m02 * m10 - m00 * m12) * inv_det;

        let i20 = (m10 * m21 - m11 * m20) * inv_det;
        let i21 = (m01 * m20 - m00 * m21) * inv_det;
        let i22 = (m00 * m11 - m01 * m10) * inv_det;

        Some(Matrix3::new(i00, i01, i02, i10, i11, i12, i20, i21, i22))
    }

    /// Multiply this matrix by a Vector3 (M * v)
    pub fn mul_vec(&self, v: Vector3f) -> Vector3f {
        Vector3f::new(
            self.c0.x * v.x + self.c1.x * v.y + self.c2.x * v.z,
            self.c0.y * v.x + self.c1.y * v.y + self.c2.y * v.z,
            self.c0.z * v.x + self.c1.z * v.y + self.c2.z * v.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    // Use nalgebra only for tests to validate our implementation.
    use nalgebra as na;

    fn to_na(m: &Matrix4f) -> na::Matrix4<f32> {
        na::Matrix4::from_column_slice(&[
            m.c0.x, m.c0.y, m.c0.z, m.c0.w, m.c1.x, m.c1.y, m.c1.z, m.c1.w, m.c2.x, m.c2.y,
            m.c2.z, m.c2.w, m.c3.x, m.c3.y, m.c3.z, m.c3.w,
        ])
    }

    fn from_na(n: &na::Matrix4<f32>) -> Matrix4f {
        Matrix4::from_slice(n.as_slice())
    }

    // Helper: transform a 4D point with our `Matrix4f` (column-major storage)
    fn transform_point_our(m: &Matrix4f, p: [f32; 4]) -> [f32; 4] {
        let x = m.c0.x * p[0] + m.c1.x * p[1] + m.c2.x * p[2] + m.c3.x * p[3];
        let y = m.c0.y * p[0] + m.c1.y * p[1] + m.c2.y * p[2] + m.c3.y * p[3];
        let z = m.c0.z * p[0] + m.c1.z * p[1] + m.c2.z * p[2] + m.c3.z * p[3];
        let w = m.c0.w * p[0] + m.c1.w * p[1] + m.c2.w * p[2] + m.c3.w * p[3];
        [x, y, z, w]
    }

    // Helper: transform a 4D point with a nalgebra matrix
    fn transform_point_na(n: &na::Matrix4<f32>, p: [f32; 4]) -> [f32; 4] {
        let v = na::Vector4::new(p[0], p[1], p[2], p[3]);
        let r = n * v;
        [r[0], r[1], r[2], r[3]]
    }

    #[test]
    fn matrix4_from_slice_roundtrip() {
        let elems: [f32; 16] = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            16.0,
        ];

        let m = Matrix4::from_slice(&elems);
        let na_m = to_na(&m);

        // Convert back and compare elements.
        let round = from_na(&na_m);

        assert_eq!(m.c0.x, round.c0.x);
        assert_eq!(m.c0.y, round.c0.y);
        assert_eq!(m.c0.z, round.c0.z);
        assert_eq!(m.c0.w, round.c0.w);
        assert_eq!(m.c1.x, round.c1.x);
        assert_eq!(m.c1.y, round.c1.y);
        assert_eq!(m.c1.z, round.c1.z);
        assert_eq!(m.c1.w, round.c1.w);
        assert_eq!(m.c2.x, round.c2.x);
        assert_eq!(m.c2.y, round.c2.y);
        assert_eq!(m.c2.z, round.c2.z);
        assert_eq!(m.c2.w, round.c2.w);
        assert_eq!(m.c3.x, round.c3.x);
        assert_eq!(m.c3.y, round.c3.y);
        assert_eq!(m.c3.z, round.c3.z);
        assert_eq!(m.c3.w, round.c3.w);
    }

    #[test]
    fn matrix4_from_trs_matches_nalgebra() {
        // translation
        let t = Vector3f::new(1.0, -2.0, 3.5);
        // construct quaternion from axis-angle via nalgebra and reuse components
        let axis = na::Vector3::new(1.0, 2.0, -1.0).normalize();
        let angle = 0.7f32;
        let na_unit = na::UnitQuaternion::from_axis_angle(&na::Unit::new_normalize(axis), angle);
        let na_q = na_unit.quaternion();
        // nalgebra quaternion is (w, i, j, k). Our Quaternionf stores (x, y, z, w).
        let r_norm = Quaternionf::new(na_q.i as f32, na_q.j as f32, na_q.k as f32, na_q.w as f32);
        let s = Vector3f::new(2.0, 0.5, -1.5);

        let our = Matrix4::from_trs(t, r_norm, s);

        // Build nalgebra equivalents: try the quaternion and its conjugate (in case of convention differences).
        let na_t = na::Translation3::new(t.x, t.y, t.z).to_homogeneous();
        let na_s = na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(s.x, s.y, s.z));

        let na_q1 = na::Quaternion::new(r_norm.w, r_norm.x, r_norm.y, r_norm.z);
        let na_q2 = na_q1.conjugate();

        let na_r1 = na::UnitQuaternion::from_quaternion(na_q1).to_homogeneous();
        let na_r2 = na::UnitQuaternion::from_quaternion(na_q2).to_homogeneous();

        let na_m1 = na_t * na_r1 * na_s;
        let na_m2 = na_t * na_r2 * na_s;

        // Test functional equivalence by transforming a set of sample points and directions.
        let samples: [[f32; 4]; 6] = [
            [0.0, 0.0, 0.0, 1.0], // origin
            [1.0, 0.0, 0.0, 1.0], // x unit
            [0.0, 1.0, 0.0, 1.0], // y unit
            [0.0, 0.0, 1.0, 1.0], // z unit
            [1.0, 1.0, 1.0, 1.0], // point
            [1.0, 2.0, 3.0, 0.0], // direction (w=0)
        ];

        let check = |na_m: &na::Matrix4<f32>| {
            for p in samples {
                let a = transform_point_our(&our, p);
                let b = transform_point_na(na_m, p);
                for i in 0..4 {
                    let diff = (a[i] - b[i]).abs();
                    if diff > EPSILON {
                        return Err((p, i, a, b, diff));
                    }
                }
            }
            Ok(())
        };

        if check(&na_m1).is_ok() {
            return;
        }
        if check(&na_m2).is_ok() {
            return;
        }

        // If neither matched, print diagnostics from na_m1 and fail.
        eprintln!("our matrix: {:?}", our);
        eprintln!("nalgebra matrix 1 columns: {:?}", na_m1.as_slice());
        eprintln!("nalgebra matrix 2 columns: {:?}", na_m2.as_slice());
        panic!("Transform mismatch against nalgebra (both quaternion conventions tried)");
    }

    #[test]
    fn vector3_transform_and_transform_normal_matches_nalgebra() {
        let t = Vector3f::new(1.0, -2.0, 3.5);
        let axis = na::Vector3::new(1.0, 2.0, -1.0).normalize();
        let angle = 0.7f32;
        let na_unit = na::UnitQuaternion::from_axis_angle(&na::Unit::new_normalize(axis), angle);
        let na_q = na_unit.quaternion();
        let r_norm = Quaternionf::new(na_q.i as f32, na_q.j as f32, na_q.k as f32, na_q.w as f32);
        let s = Vector3f::new(2.0, 0.5, -1.5);

        let our = Matrix4::from_trs(t, r_norm, s);
        let na_m = to_na(&our);

        // Test point transforms via Vector3::transform against nalgebra (with perspective divide)
        let point_samples: [[f32; 4]; 8] = [
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [-1.0, 2.5, -0.5, 1.0],
            [0.123, -0.456, 2.0, 1.0],
            [100.0, -50.0, 2.0, 1.0],
        ];

        for p in point_samples.iter() {
            let ours = Vector3f::new(p[0], p[1], p[2]).transform(&our);
            let na_res = transform_point_na(&na_m, *p);
            let expected = if na_res[3].abs() > 1e-6 {
                [na_res[0] / na_res[3], na_res[1] / na_res[3], na_res[2] / na_res[3]]
            } else {
                [na_res[0], na_res[1], na_res[2]]
            };
            assert!((ours.x - expected[0]).abs() < EPSILON, "point x mismatch: {} vs {}", ours.x, expected[0]);
            assert!((ours.y - expected[1]).abs() < EPSILON, "point y mismatch: {} vs {}", ours.y, expected[1]);
            assert!((ours.z - expected[2]).abs() < EPSILON, "point z mismatch: {} vs {}", ours.z, expected[2]);
        }

        // Test normal transforms via Vector3::transform_normal against nalgebra (inverse-transpose of linear 3x3)
        let normal_samples: [Vector3f; 6] = [
            Vector3f::new(1.0, 0.0, 0.0),
            Vector3f::new(0.0, 1.0, 0.0),
            Vector3f::new(0.0, 0.0, 1.0),
            Vector3f::new(1.0, 1.0, 0.0),
            Vector3f::new(1.0, -1.0, 2.0),
            Vector3f::new(0.123, -0.456, 0.789),
        ];

        let na_lin = na_m.fixed_view::<3, 3>(0, 0).into_owned();
        let na_inv = na_lin
            .try_inverse()
            .expect("nalgebra linear part should be invertible for this test");
        let na_inv_t = na_inv.transpose();

        for n in normal_samples.iter() {
            let ours = n.transform_normal(&our);
            let nv = na::Vector3::new(n.x, n.y, n.z);
            let b = na_inv_t * nv;
            assert!((ours.x - b[0]).abs() < EPSILON, "normal x mismatch: {} vs {}", ours.x, b[0]);
            assert!((ours.y - b[1]).abs() < EPSILON, "normal y mismatch: {} vs {}", ours.y, b[1]);
            assert!((ours.z - b[2]).abs() < EPSILON, "normal z mismatch: {} vs {}", ours.z, b[2]);
        }
    }

    #[test]
    fn vector3_basic_ops_matches_nalgebra() {
        let eps = 1e-6f32;

        let samples: [([f32; 3], [f32; 3]); 4] = [
            ([1.0, 2.0, 3.0], [4.0, -1.0, 0.5]),
            ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
            ([-1.0, 2.5, -0.5], [0.123, -0.456, 2.0]),
            ([10.0, -5.0, 2.0], [-2.0, 3.0, 0.25]),
        ];

        for (a, b) in samples.iter() {
            let our_a = Vector3f::new(a[0], a[1], a[2]);
            let our_b = Vector3f::new(b[0], b[1], b[2]);

            let their_a = na::Vector3::new(a[0], a[1], a[2]);
            let their_b = na::Vector3::new(b[0], b[1], b[2]);

            // add
            let our_add = our_a + our_b;
            let their_add = their_a + their_b;
            assert!((our_add.x - their_add[0]).abs() < eps);
            assert!((our_add.y - their_add[1]).abs() < eps);
            assert!((our_add.z - their_add[2]).abs() < eps);

            // sub
            let our_sub = our_a - our_b;
            let their_sub = their_a - their_b;
            assert!((our_sub.x - their_sub[0]).abs() < eps);
            assert!((our_sub.y - their_sub[1]).abs() < eps);
            assert!((our_sub.z - their_sub[2]).abs() < eps);

            // scalar mul
            let scalar = 2.5f32;
            let our_mul = our_a * scalar;
            let their_mul = their_a * scalar;
            assert!((our_mul.x - their_mul[0]).abs() < eps);
            assert!((our_mul.y - their_mul[1]).abs() < eps);
            assert!((our_mul.z - their_mul[2]).abs() < eps);

            // dot
            let our_dot = our_a.dot(&our_b);
            let their_dot = their_a.dot(&their_b);
            assert!((our_dot - their_dot).abs() < eps);

            // cross
            let our_cross = our_a.cross(&our_b);
            let their_cross = their_a.cross(&their_b);
            assert!((our_cross.x - their_cross[0]).abs() < eps);
            assert!((our_cross.y - their_cross[1]).abs() < eps);
            assert!((our_cross.z - their_cross[2]).abs() < eps);

            // norm and normalize
            let our_norm = our_a.norm();
            let their_norm = their_a.norm();
            assert!((our_norm - their_norm).abs() < 1e-5);

            let our_n = our_a.normalize();
            let their_n = their_a.normalize();
            // nalgebra may produce non-finite values when normalizing a zero vector.
            // Only compare normalized vectors when their norm is meaningful; otherwise
            // expect our normalizer to return the zero vector.
            let their_norm_val = their_a.norm();
            if their_norm_val > 1e-6 {
                assert!((our_n.x - their_n[0]).abs() < 1e-5);
                assert!((our_n.y - their_n[1]).abs() < 1e-5);
                assert!((our_n.z - their_n[2]).abs() < 1e-5);
            } else {
                assert!((our_n.x).abs() < 1e-6);
                assert!((our_n.y).abs() < 1e-6);
                assert!((our_n.z).abs() < 1e-6);
            }

            // distance
            let our_dist = our_a.distance(&our_b);
            let their_dist = (their_a - their_b).norm();
            assert!((our_dist - their_dist).abs() < 1e-5);
        }
    }

    #[test]
    fn matrix3_inverse_transpose_mulvec_matches_nalgebra() {
        // Build a deterministic, invertible 3x3 matrix
        let our_m = Matrix3::new(
            3.0, 1.0, 2.0, // first row
            2.0, 4.0, 1.0, // second row
            0.5, 1.0, 3.0, // third row
        );

        let their_m = na::Matrix3::from_column_slice(&[
            our_m.c0.x, our_m.c0.y, our_m.c0.z, our_m.c1.x, our_m.c1.y, our_m.c1.z, our_m.c2.x,
            our_m.c2.y, our_m.c2.z,
        ]);

        let our_t = our_m.transpose();
        let their_t = their_m.transpose();
        let eps = 1e-6f32;

        // compare transpose entries
        let our_cols = [our_t.c0, our_t.c1, our_t.c2];
        for c in 0..3 {
            let col = our_cols[c];
            let their_col = their_t.column(c);
            assert!((col.x - their_col[0]).abs() < eps);
            assert!((col.y - their_col[1]).abs() < eps);
            assert!((col.z - their_col[2]).abs() < eps);
        }

        // inverse
        let our_inv = our_m.inverse().expect("our inverse");
        let their_inv = their_m
            .try_inverse()
            .expect("their inverse");

        let our_cols = [our_inv.c0, our_inv.c1, our_inv.c2];
        for c in 0..3 {
            let col = our_cols[c];
            let their_col = their_inv.column(c);
            assert!((col.x - their_col[0]).abs() < 1e-5);
            assert!((col.y - their_col[1]).abs() < 1e-5);
            assert!((col.z - their_col[2]).abs() < 1e-5);
        }

        // mul_vec
        let v = Vector3f::new(1.0, -2.0, 0.5);
        let their_v = na::Vector3::new(v.x, v.y, v.z);
        let our_res = our_m.mul_vec(v);
        let their_res = their_m * their_v; // compare same linear transform M * v
        assert!((our_res.x - their_res[0]).abs() < 1e-5);
        assert!((our_res.y - their_res[1]).abs() < 1e-5);
        assert!((our_res.z - their_res[2]).abs() < 1e-5);
    }

    #[test]
    fn quaternion_from_rotation_matches_nalgebra_via_transforms() {
        // Create a rotation via nalgebra and compare transforms produced
        let axis = na::Vector3::new(0.3, 0.7, -0.2).normalize();
        let angle = 1.2345f32;
        let their_unit = na::UnitQuaternion::from_axis_angle(&na::Unit::new_normalize(axis), angle);

        // build our Matrix3 from their rotation matrix
        let their_r3 = their_unit.to_rotation_matrix().matrix().into_owned();
        let our_m3 = Matrix3::new(
            their_r3[(0, 0)], their_r3[(0, 1)], their_r3[(0, 2)],
            their_r3[(1, 0)], their_r3[(1, 1)], their_r3[(1, 2)],
            their_r3[(2, 0)], their_r3[(2, 1)], their_r3[(2, 2)],
        );

        let our_q = Quaternionf::from_rotation(&our_m3);

        // Construct homogeneous transforms and compare applying them to sample points
        let our_m4 = Matrix4::from_trs(Vector3f::new(0.0, 0.0, 0.0), our_q, Vector3f::new(1.0, 1.0, 1.0));
        let their_m4 = their_unit.to_homogeneous();

        let samples: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [1.0, 2.0, -1.0, 1.0],
        ];

        let eps = 1e-5f32;
        for p in samples.iter() {
            let our_tr = transform_point_our(&our_m4, *p);
            let their_tr = transform_point_na(&their_m4, *p);
            for i in 0..4 {
                assert!((our_tr[i] - their_tr[i]).abs() < eps, "quat transform mismatch idx {}: {} vs {}", i, our_tr[i], their_tr[i]);
            }
        }
    }
}
