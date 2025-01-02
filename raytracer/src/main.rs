use fps_ticker::Fps;
use nannou::prelude::*;
use nannou::winit::event::{ElementState, MouseButton, VirtualKeyCode, WindowEvent};
use nannou_egui::{self, egui, Egui};
use shared::*;
use spirv_builder::{Capability, MetadataPrintout, SpirvBuilder};
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;

const WIN_WIDTH: u32 = 800;
const WIN_HEIGHT: u32 = 600;

fn main() {
    nannou::app(model).update(update).run();
}

struct Model {
    pipeline: wgpu::RenderPipeline,
    gui: Egui,
    fps: Fps,
    hold_pos: Option<Point2>,
    camera: Camera,
    bounce_limit: u32,
    time: u32,
    mouse_speed: f32,
    move_speed: f32,
    background: [f32; 3],
    current_pressed_keys: HashSet<VirtualKeyCode>,
}

fn model(_app: &App) -> Model {
    let device_description = wgpu::DeviceDescriptor {
        label: Some("device desc"),
        features: wgpu::Features::PUSH_CONSTANTS,
        limits: wgpu::Limits {
            max_push_constant_size: 256,
            ..Default::default()
        },
    };

    let win_id = _app
        .new_window()
        .title("Locked in raytracer")
        .device_descriptor(device_description)
        .size(WIN_WIDTH, WIN_HEIGHT)
        .resizable(false)
        .view(view)
        .raw_event(raw_event_func)
        .build()
        .unwrap();

    let window = _app.window(win_id).unwrap();
    let gui = Egui::from_window(&window);
    let device = window.device();

    let shader_module = device.create_shader_module(load_shader_desc());

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::all(),
            range: 0..std::mem::size_of::<ShaderConsts>() as u32,
        }],
    });

    let pipeline = wgpu::RenderPipelineBuilder::from_layout(&pipeline_layout, &shader_module)
        .fragment_shader(&shader_module)
        .vertex_entry_point("main_vs")
        .fragment_entry_point("main_fs")
        .sample_count(window.msaa_samples())
        .build(device);

    Model {
        pipeline,
        gui,
        fps: Fps::default(),
        hold_pos: None,
        bounce_limit: 5,
        time: 0,
        mouse_speed: 20.0,
        move_speed: 30.0,
        background: [0.0, 0.0, 0.0],
        current_pressed_keys: HashSet::new(),
        camera: Camera::new(
            WIN_WIDTH as f32,
            WIN_HEIGHT as f32,
            50,
            90.0,
            glam::Vec3::new(0.0, 1.0, 2.0),
            -90.0,
            0.0,
        ),
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let egui = &mut model.gui;

    egui.set_elapsed_time(_update.since_start);
    let ctx = egui.begin_frame();

    egui::Window::new("Settings").show(&ctx, |ui| {
        ui.label("Move with WASD, space, shift");
        ui.label("Drag mouse to look around");

        ui.add_space(15.0);
        ui.label(format!("FPS: {:.2}", model.fps.avg()));
        ui.label(format!("FPS min: {:.2}", model.fps.min()));
        ui.label(format!("FPS max: {:.2}", model.fps.max()));
        ui.add_space(15.0);

        ui.label("Samples");
        ui.add(egui::Slider::new(&mut model.camera.samples, 1..=1000));

        ui.label("Bounce limit");
        ui.add(egui::Slider::new(&mut model.bounce_limit, 1..=20));

        ui.label("FOV");
        ui.add(egui::Slider::new(&mut model.camera.fov, 1.0..=150.0));

        ui.label("Move speed");
        ui.add(egui::Slider::new(&mut model.move_speed, 1.0..=100.0));

        ui.label("Mouse speed");
        ui.add(egui::Slider::new(&mut model.mouse_speed, 1.0..=100.0));

        ui.label("Background color");
        ui.color_edit_button_rgb(&mut model.background);
    });

    if !model.current_pressed_keys.is_empty() {
        let forward_vector = model.camera.direction();
        let right_vector = forward_vector.cross(glam::Vec3::new(0.0, 1.0, 0.0));
        let up_vector = glam::Vec3::new(0.0, 1.0, 0.0);
        let move_speed_offset = 0.001;

        for key in model.current_pressed_keys.iter() {
            match key {
                VirtualKeyCode::W => {
                    model.camera.pos += forward_vector * model.move_speed * move_speed_offset;
                }
                VirtualKeyCode::A => {
                    model.camera.pos -= right_vector * model.move_speed * move_speed_offset;
                }
                VirtualKeyCode::S => {
                    model.camera.pos -= forward_vector * model.move_speed * move_speed_offset;
                }
                VirtualKeyCode::D => {
                    model.camera.pos += right_vector * model.move_speed * move_speed_offset;
                }
                VirtualKeyCode::Space => {
                    model.camera.pos += up_vector * model.move_speed * move_speed_offset;
                }
                VirtualKeyCode::LShift => {
                    model.camera.pos -= up_vector * model.move_speed * move_speed_offset;
                }
                _ => {}
            }
        }
    }

    model.time += 1;
}

fn view(app: &App, model: &Model, frame: Frame) {
    let mut encoder = frame.command_encoder();

    let mut render_pass = wgpu::RenderPassBuilder::new()
        .color_attachment(frame.texture_view(), |color| color)
        .begin(&mut encoder);
    render_pass.set_pipeline(&model.pipeline);

    let constants = ShaderConsts {
        time: model.time,
        bounce_limit: model.bounce_limit,
        width: model.camera.width,
        height: model.camera.height,
        samples: model.camera.samples,
        fov: model.camera.fov,
        pos: (model.camera.pos.x, model.camera.pos.y, model.camera.pos.z),
        yaw: model.camera.yaw,
        pitch: model.camera.pitch,
        background: (
            model.background[0],
            model.background[1],
            model.background[2],
        ),
    };

    let bytes = unsafe { any_as_u8_slice(&constants) };
    render_pass.set_push_constants(wgpu::ShaderStages::all(), 0, bytes);

    render_pass.draw(0..3, 0..1);

    drop(render_pass);
    drop(encoder);

    model.gui.draw_to_frame(&frame);
    model.fps.tick();
}

fn raw_event_func(app: &App, model: &mut Model, event: &WindowEvent) {
    model.gui.handle_raw_event(event);

    if let WindowEvent::MouseInput { button, state, .. } = event {
        if *button == MouseButton::Left {
            match state {
                ElementState::Pressed => {
                    model.hold_pos = Some(vec2(
                        app.mouse.position().x + (WIN_WIDTH as f32 / 2.0),
                        -app.mouse.position().y + (WIN_HEIGHT as f32 / 2.0),
                    ));
                }
                ElementState::Released => {
                    model.hold_pos = None;
                }
            }
        }
    }

    if let WindowEvent::CursorMoved { position, .. } = event {
        if let Some(pos) = model.hold_pos {
            let dir = vec2(position.x as f32, position.y as f32) - pos;
            let yaw = dir.x * model.mouse_speed * 0.01;
            let pitch = -dir.y * model.mouse_speed * 0.01;
            model.camera.yaw += yaw;
            model.camera.pitch += pitch;
            model.camera.pitch = model.camera.pitch.clamp(-89.0, 89.0);

            model.hold_pos = Some(vec2(position.x as f32, position.y as f32));
        }
    }

    if let WindowEvent::KeyboardInput { input, .. } = event {
        if let Some(keycode) = input.virtual_keycode {
            match input.state {
                ElementState::Pressed => {
                    model.current_pressed_keys.insert(keycode);
                }
                ElementState::Released => {
                    model.current_pressed_keys.remove(&keycode);
                }
            }
        }
    }
}

// From https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts((p as *const T) as *const u8, std::mem::size_of::<T>())
}

fn load_shader_desc() -> wgpu::ShaderModuleDescriptor<'static> {
    let crate_path = [env!("CARGO_MANIFEST_DIR"), "..", "shaders"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let compile_res = SpirvBuilder::new(crate_path, "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::None)
        .capability(Capability::Int8)
        .build()
        .unwrap();

    let data = std::fs::read(compile_res.module.unwrap_single()).unwrap();
    let spirv = wgpu::util::make_spirv(&data);

    let source = match spirv {
        wgpu::ShaderSource::SpirV(cow) => wgpu::ShaderSource::SpirV(Cow::Owned(cow.into_owned())),
        _ => panic!("Unexpected shader source"),
    };
    wgpu::ShaderModuleDescriptor {
        label: Some("shader desc"),
        source,
    }
}
