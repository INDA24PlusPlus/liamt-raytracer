use nannou::prelude::*;
use shared::*;
use spirv_builder::{Capability, MetadataPrintout, SpirvBuilder};
use std::borrow::Cow;
use std::path::PathBuf;

const WIN_WIDTH: u32 = 800;
const WIN_HEIGHT: u32 = 600;

fn main() {
    nannou::app(model).update(update).run();
}

struct Model {
    pipeline: wgpu::RenderPipeline,
    constants: ShaderConsts,
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
        .build()
        .unwrap();
    let window = _app.window(win_id).unwrap();
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

    let constants = ShaderConsts {
        resolution: [WIN_WIDTH, WIN_HEIGHT],
        time: 0.0,
    };

    Model {
        pipeline,
        constants,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    model.constants.time += 1.0;
}

fn view(app: &App, model: &Model, frame: Frame) {
    let mut encoder = frame.command_encoder();

    let mut render_pass = wgpu::RenderPassBuilder::new()
        .color_attachment(frame.texture_view(), |color| color)
        .begin(&mut encoder);
    render_pass.set_pipeline(&model.pipeline);

    let bytes = unsafe { any_as_u8_slice(&model.constants) };
    render_pass.set_push_constants(wgpu::ShaderStages::all(), 0, bytes);

    render_pass.draw(0..3, 0..1);
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
