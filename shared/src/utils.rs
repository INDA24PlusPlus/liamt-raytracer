use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use spirv_std::num_traits::Float;

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Color(Vec3);

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self(vec3(r, g, b))
    }

    pub fn r(&self) -> f32 {
        self.0.x
    }

    pub fn g(&self) -> f32 {
        self.0.y
    }

    pub fn b(&self) -> f32 {
        self.0.z
    }

    pub fn to_srgb(&self) -> Vec3 {
        vec3(
            self.0.x.powf(1.0 / 2.2),
            self.0.y.powf(1.0 / 2.2),
            self.0.z.powf(1.0 / 2.2),
        )
    }
}

impl core::ops::Add for Color {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl core::ops::AddAssign for Color {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl core::ops::Mul for Color {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl core::ops::MulAssign for Color {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl core::ops::Mul<f32> for Color {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self(self.0 * scalar)
    }
}

impl core::ops::MulAssign<f32> for Color {
    fn mul_assign(&mut self, scalar: f32) {
        self.0 *= scalar;
    }
}

impl core::ops::Mul<Color> for f32 {
    type Output = Color;

    fn mul(self, color: Color) -> Color {
        Color(color.0 * self)
    }
}
impl core::ops::Div for Color {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self(self.0 / other.0)
    }
}

impl core::ops::DivAssign for Color {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
    }
}

impl core::ops::Div<f32> for Color {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        Self(self.0 / scalar)
    }
}

impl core::ops::DivAssign<f32> for Color {
    fn div_assign(&mut self, scalar: f32) {
        self.0 /= scalar;
    }
}
