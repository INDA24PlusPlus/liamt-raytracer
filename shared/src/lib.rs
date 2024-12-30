#![no_std]

use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use spirv_std::num_traits::Float;
use spirv_std::vector::Vector;

pub struct ShaderConsts {
    pub resolution: [u32; 2],
    pub time: f32,
}

pub struct RNG {
    pub state: u32,
}

impl RNG {
    pub fn new(consts: &ShaderConsts, coords: Vec4) -> Self {
        // idk what im doing here
        let mut state = consts.resolution[0].wrapping_mul(coords.x as u32);
        state ^= consts.resolution[1].wrapping_mul(coords.y as u32);
        state ^= consts.time.to_bits();

        Self { state }
    }

    // PCG random num gen. From https://github.com/JMS55/bevy/blob/solari3/crates/bevy_pbr/src/solari/global_illumination/utils.wgsl#L8-L18
    pub fn rand_u(&mut self) -> u32 {
        self.state = self.state * 747796405 + 2891336453;
        let word = ((self.state >> ((self.state >> 28) + 4)) ^ self.state) * 277803737;
        (word >> 22) ^ word
    }

    pub fn rand_f(&mut self) -> f32 {
        let word = self.rand_u();
        f32::from_bits((word >> 9) | 0x3f800000) - 1.0
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct HitData {
    pub point: Vec3,
    pub normal: Vec3,
    pub t: f32,
    pub front: bool,
}

impl HitData {
    pub fn new() -> Self {
        Self {
            point: vec3(0.0, 0.0, 0.0),
            normal: vec3(0.0, 0.0, 0.0),
            t: 0.0,
            front: false,
        }
    }

    pub fn set_normal(&mut self, ray: &Ray, out_normal: Vec3) {
        self.front = ray.direction.dot(out_normal) < 0.0;
        self.normal = if self.front { out_normal } else { -out_normal };
    }
}

pub trait Hittable {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32, hit_data: &mut HitData) -> bool;
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32, hit_data: &mut HitData) -> bool {
        // Calculate the discriminant
        let oc = ray.origin - self.center;
        let a = ray.direction.length_squared();
        let half_b = oc.dot(ray.direction);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;

        // No roots = no hit
        if discriminant < 0.0 {
            return false;
        }
        let sqrt = discriminant.sqrt();
        let mut root = (-half_b - sqrt) / a;

        // Take the intersection root
        if root < t_min || t_max < root {
            root = (-half_b + sqrt) / a;
            if root < t_min || t_max < root {
                return false;
            }
        }

        // Found a hit, set hit data
        hit_data.t = root;
        hit_data.point = ray.at(hit_data.t);
        hit_data.normal = (hit_data.point - self.center) / self.radius;
        let out_normal = (hit_data.point - self.center) / self.radius;
        hit_data.set_normal(ray, out_normal);

        true
    }
}

// Very hacky thing i found, but it like works so im happy
// Using this because vectors not work and like i cant pass lists as arguments
impl<T: Copy + Hittable, const N: usize> Hittable for [T; N] {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32, hit_data: &mut HitData) -> bool {
        let mut data = HitData::new();
        let mut has_hit = false;
        let mut closest = t_max;

        for i in 0..N {
            if self[i].hit(ray, t_min, closest, &mut data) {
                has_hit = true;
                closest = data.t;
                *hit_data = data;
            }
        }

        has_hit
    }
}

pub fn ray_color(ray: &Ray, world: impl Copy + Hittable) -> Vec4 {
    let mut hit_data = HitData::new();

    if world.hit(ray, 0.0, f32::INFINITY, &mut hit_data) {
        // Nice rainbow colors
        return vec4(
            0.5 * (hit_data.normal.x + 1.0),
            0.5 * (hit_data.normal.y + 1.0),
            0.5 * (hit_data.normal.z + 1.0),
            1.0,
        );
    }

    // Nice sky gradient color
    let res = 0.5 * (ray.direction.normalize().y + 1.0);
    vec4(
        (1.0 - res) * 1.0 + res * 0.5,
        (1.0 - res) * 1.0 + res * 0.7,
        (1.0 - res) * 1.0 + res * 1.0,
        1.0,
    )
}

pub struct Camera {
    pub aspect_ratio: f32,
    pub width: f32,
    pub height: f32,
    pub center: Vec3,
    pub first: Vec3,
    pub pdu: Vec3,
    pub pdv: Vec3,
    pub samples: u32,
}

impl Camera {
    pub fn new(width: f32, height: f32, focal_len: f32, vph: f32, samples: u32) -> Self {
        let center = vec3(0.0, 0.0, 0.0);

        let aspect_ratio = width / height;
        let vpw = vph * aspect_ratio;

        let vpu = vec3(vpw, 0.0, 0.0);
        let vpv = vec3(0.0, -vph, 0.0);

        let pdu = vpu / width;
        let pdv = vpv / height;

        let vp_start = center - vec3(0.0, 0.0, focal_len) - vpu / 2.0 - vpv / 2.0;

        let first = vp_start + pdu / 2.0 + pdv / 2.0;

        Self {
            aspect_ratio,
            width,
            height,
            center,
            first,
            pdu,
            pdv,
            samples,
        }
    }
}
