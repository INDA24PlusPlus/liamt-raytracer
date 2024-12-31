#![no_std]

use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use spirv_std::num_traits::Float;
use spirv_std::num_traits::FloatConst;
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
        let mut state = coords.x.to_bits() + coords.y.to_bits() * consts.resolution[0];
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

    pub fn rand_f_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.rand_f() * (max - min)
    }

    pub fn rand_vec3(&mut self) -> Vec3 {
        vec3(self.rand_f(), self.rand_f(), self.rand_f())
    }

    pub fn rand_vec3_range(&mut self, min: f32, max: f32) -> Vec3 {
        vec3(
            self.rand_f_range(min, max),
            self.rand_f_range(min, max),
            self.rand_f_range(min, max),
        )
    }

    pub fn rand_unit_vec3(&mut self) -> Vec3 {
        let a = self.rand_f_range(0.0, 2.0 * f32::PI());
        let z = self.rand_f_range(-1.0, 1.0);
        let r = (1.0 - z * z).sqrt();
        vec3(r * a.cos(), r * a.sin(), z)
    }

    pub fn rand_hemisphere_vec3(&mut self, normal: Vec3) -> Vec3 {
        let on = self.rand_unit_vec3();
        if on.dot(normal) > 0.0 {
            on
        } else {
            -on
        }
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
    pub material: Material,
}

impl HitData {
    pub fn new() -> Self {
        Self {
            point: vec3(0.0, 0.0, 0.0),
            normal: vec3(0.0, 0.0, 0.0),
            t: 0.0,
            front: false,
            material: Material::new(vec3(0.0, 0.0, 0.0), 0.0),
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
    pub material: Material,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            radius,
            material: Material::new(vec3(0.0, 0.0, 0.0), 0.0),
        }
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
        hit_data.material = self.material;

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

pub fn ray_color(mut ray: Ray, world: impl Copy + Hittable, rng: &mut RNG, max_depth: u32) -> Vec3 {
    let mut color = vec3(1.0, 1.0, 1.0);

    for _ in 0..max_depth {
        let mut hit_data = HitData::new();

        if world.hit(&ray, 0.0001, f32::INFINITY, &mut hit_data) {
            let (r, col) = hit_data.material.scatter(&ray, &hit_data, rng);
            ray = r;
            color *= col;
        } else {
            let res = 0.5 * (ray.direction.normalize().y + 1.0);
            return color
                * vec3(
                    (1.0 - res) * 1.0 + res * 0.3,
                    (1.0 - res) * 1.0 + res * 0.5,
                    (1.0 - res) * 1.0 + res * 1.0,
                );
        }
    }

    // If max depth reached
    vec3(0.0, 0.0, 0.0)
}

pub fn convert_color(color: f32) -> f32 {
    if color > 0.0 {
        color.sqrt()
    } else {
        0.0
    }
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

        // Viewport vectors
        let vpu = vec3(vpw, 0.0, 0.0);
        let vpv = vec3(0.0, -vph, 0.0);

        // Pixel delta vectors
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

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Material {
    pub color: Vec3,
    pub shininess: f32,
}

impl Material {
    pub fn new(color: Vec3, shininess: f32) -> Self {
        Self { color, shininess }
    }
    pub fn scatter(&self, ray: &Ray, hit_data: &HitData, rng: &mut RNG) -> (Ray, Vec3) {
        let scatter_dir = if rng.rand_f() < self.shininess {
            ray.direction.reflect(hit_data.normal)
        } else {
            hit_data.normal + rng.rand_unit_vec3()
        };

        let ray = Ray::new(hit_data.point, scatter_dir);
        let att = self.color;
        (ray, att)
    }
}
