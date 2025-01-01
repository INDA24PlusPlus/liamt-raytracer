#![cfg_attr(target_arch = "spirv", no_std)]

use shared::*;
use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use spirv_std::macros::spirv;

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(frag_coord)] in_coord: Vec4,
    #[spirv(push_constant)] constants: &ShaderConsts,
    output: &mut Vec4,
) {
    let camera = Camera::new(
        constants.resolution[0] as f32,
        constants.resolution[1] as f32,
        100,
        90.0,
        vec3(-2.0, 2.0, 1.0),
        vec3(0.0, 0.0, -1.0),
        vec3(0.0, 1.0, 0.0),
    );

    let max_depth = 3;
    let background = vec3(0.0, 0.0, 0.0);

    let mut rng = RNG::new(constants, in_coord);

    let redmat = Material {
        color: vec3(1.0, 0.0, 0.0),
        shininess: 0.0,
        emission: 0.0,
    };
    let greenmat = Material {
        color: vec3(0.0, 1.0, 0.0),
        shininess: 0.0,
        emission: 0.0,
    };
    let bluemat = Material {
        color: vec3(0.0, 0.0, 1.0),
        shininess: 0.0,
        emission: 0.0,
    };
    let shinymat = Material {
        color: vec3(1.0, 1.0, 1.0),
        shininess: 1.0,
        emission: 0.0,
    };
    let lightmat = Material {
        color: vec3(1.0, 1.0, 1.0),
        shininess: 0.0,
        emission: 1.0,
    };

    let world = [
        Sphere {
            center: vec3(0.0, 0.0, -1.0),
            radius: 0.5,
            material: shinymat,
        },
        Sphere {
            center: vec3(1.3, 0.0, -1.0),
            radius: 0.5,
            material: lightmat,
        },
        Sphere {
            center: vec3(-2.0, 1.0, -2.0),
            radius: 0.3,
            material: greenmat,
        },
        Sphere {
            center: vec3(2.5, 0.5, -3.5),
            radius: 0.2,
            material: bluemat,
        },
        Sphere {
            center: vec3(0.0, -100.5, -1.0),
            radius: 100.0,
            material: redmat,
        },
    ];

    let mut color = vec3(0.0, 0.0, 0.0);

    for _ in 0..camera.samples {
        let offset_square = vec3(rng.rand_f() - 0.5, rng.rand_f() - 0.5, 0.0);
        let pixel_center = camera.first
            + camera.pdu * (in_coord.x + offset_square.x)
            + camera.pdv * (in_coord.y + offset_square.y);
        let ray_direction = pixel_center - camera.center;
        let ray = Ray::new(camera.center, ray_direction);

        color += ray_color(ray, world, &mut rng, max_depth, background);
    }

    color /= camera.samples as f32;

    *output = vec4(color.x, color.y, color.z, 1.0);
}

#[spirv(vertex)]
pub fn main_vs(#[spirv(vertex_index)] idx: i32, #[spirv(position)] position: &mut Vec4) {
    // From https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/
    let pos = 2.0 * vec2(((idx << 1) & 2) as f32, (idx & 2) as f32) - Vec2::ONE;
    *position = pos.extend(0.0).extend(1.0);
}
