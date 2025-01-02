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
        constants.width,
        constants.height,
        constants.samples,
        constants.fov,
        vec3(constants.pos.0, constants.pos.1, constants.pos.2),
        constants.yaw,
        constants.pitch,
    );

    let max_depth = constants.bounce_limit;
    let background = vec3(0.0, 0.0, 0.0);

    let mut rng = RandomSauce::new(constants, in_coord);

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

    let spheres = [
        Sphere {
            center: vec3(0.0, 0.5, -1.0),
            radius: 0.5,
            material: shinymat,
        },
        Sphere {
            center: vec3(1.3, 0.5, -1.0),
            radius: 0.5,
            material: lightmat,
        },
        Sphere {
            center: vec3(-2.0, 1.5, -2.0),
            radius: 0.3,
            material: greenmat,
        },
        Sphere {
            center: vec3(2.5, 1.0, -3.5),
            radius: 0.2,
            material: bluemat,
        },
    ];

    let planes = [Plane {
        y: 0.0,
        material: redmat,
    }];

    let mut color = vec3(0.0, 0.0, 0.0);

    let pdu = camera.pdu();
    let pdv = camera.pdv();
    let first = camera.first();

    for _ in 0..camera.samples {
        let offset_x = rng.rand_f() - 0.5;
        let offset_y = rng.rand_f() - 0.5;

        let pixel_center =
            first + pdu * (in_coord.x + offset_x) + pdv * (camera.height - (in_coord.y + offset_y));
        let ray_direction = pixel_center - camera.pos;
        let ray = Ray::new(camera.pos, ray_direction);

        color += ray_color(ray, &spheres, &planes, &mut rng, max_depth, background);
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
