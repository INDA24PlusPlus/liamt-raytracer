#![no_std]

use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use spirv_std::num_traits::Float;
use spirv_std::num_traits::FloatConst;

#[repr(C)]
pub struct ShaderConsts {
    pub bounce_limit: u32,
    pub time: u32,
    pub width: f32,
    pub height: f32,
    pub samples: u32,
    pub fov: f32,
    pub pos: (f32, f32, f32),
    pub yaw: f32,
    pub pitch: f32,
}

pub struct RandomSauce {
    pub state: u32,
}

impl RandomSauce {
    pub fn new(consts: &ShaderConsts, coords: Vec4) -> Self {
        let mut state = coords.x.to_bits() + coords.y.to_bits() * 10000;
        state ^= consts.time * 1337;
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
            material: Material {
                color: vec3(0.0, 0.0, 0.0),
                shininess: 0.0,
                emission: 0.0,
            },
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

pub fn ray_color(
    mut ray: Ray,
    world: impl Copy + Hittable,
    rng: &mut RandomSauce,
    max_depth: u32,
    background: Vec3,
) -> Vec3 {
    let mut color = vec3(1.0, 1.0, 1.0);
    let mut light = vec3(0.0, 0.0, 0.0);

    for _ in 0..max_depth {
        let mut hit_data = HitData::new();
        if world.hit(&ray, 0.0001, f32::INFINITY, &mut hit_data) {
            let (r, col) = hit_data.material.scatter(&ray, &hit_data, rng);
            ray = r;
            let emit = hit_data.material.emit();
            light += color * emit;
            color *= col;
        } else {
            light += color * background;
            break;
        }
    }

    light
}

pub fn convert_color(color: f32) -> f32 {
    if color > 0.0 {
        color.sqrt()
    } else {
        0.0
    }
}

pub struct Camera {
    pub width: f32,
    pub height: f32,
    pub pos: Vec3,
    pub samples: u32,
    pub fov: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Camera {
    pub fn new(
        width: f32,
        height: f32,
        samples: u32,
        fov: f32,
        pos: Vec3,
        yaw: f32,
        pitch: f32,
    ) -> Self {
        Self {
            width,
            height,
            samples,
            fov,
            yaw,
            pitch,
            pos,
        }
    }

    pub fn direction(&self) -> Vec3 {
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();
        vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
    }
    pub fn u(&self) -> Vec3 {
        let forward = self.direction();
        let up = vec3(0.0, 1.0, 0.0);
        forward.cross(up).normalize()
    }
    pub fn v(&self) -> Vec3 {
        self.u().cross(self.direction()).normalize()
    }
    pub fn near_plane_dimensions(&self) -> (f32, f32) {
        let plane_height = 2.0 * (self.fov.to_radians() / 2.0).tan();
        let plane_width = plane_height * (self.width / self.height);
        (plane_width, plane_height)
    }
    pub fn first(&self) -> Vec3 {
        let (plane_width, plane_height) = self.near_plane_dimensions();
        let center = self.pos + self.direction();
        let right = self.u().normalize() * (plane_width * 0.5);
        let up = self.v().normalize() * (plane_height * 0.5);
        center - right - up
    }
    pub fn pdu(&self) -> Vec3 {
        let (plane_width, _) = self.near_plane_dimensions();
        self.u().normalize() * (plane_width / self.width)
    }
    pub fn pdv(&self) -> Vec3 {
        let (_, plane_height) = self.near_plane_dimensions();
        self.v().normalize() * (plane_height / self.height)
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Material {
    pub color: Vec3,
    pub shininess: f32,
    pub emission: f32,
}

impl Material {
    pub fn scatter(&self, ray: &Ray, hit_data: &HitData, rng: &mut RandomSauce) -> (Ray, Vec3) {
        let scatter_dir = if rng.rand_f() < self.shininess {
            ray.direction.reflect(hit_data.normal)
        } else {
            hit_data.normal + rng.rand_unit_vec3()
        };

        let ray = Ray::new(hit_data.point, scatter_dir);
        let att = self.color;
        (ray, att)
    }
    pub fn emit(&self) -> Vec3 {
        self.color * self.emission
    }
}
