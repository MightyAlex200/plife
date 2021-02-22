mod serialize;
mod simulation;
mod util;
mod visualization;

use std::{
    fs::File,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use simulation::*;
use structopt::StructOpt;
use visualization::*;
use wgpu::*;
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

#[derive(StructOpt)]
/// Particle life simulator
struct Args {
    config_file: PathBuf,
    #[structopt(long)]
    headless: bool,
    #[structopt(long)]
    checkpoint: Option<u64>,
    #[structopt(long)]
    steps: Option<u64>,
}

#[paw::main]
fn main(args: Args) {
    futures::executor::block_on(main_async(args));
}

async fn main_async(args: Args) {
    let Args {
        config_file,
        headless,
        checkpoint,
        steps,
    } = args;
    let instance = Instance::new(BackendBit::all());

    let window_stuff = if headless {
        None
    } else {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_resizable(true)
            .with_title("plife visualization")
            .with_inner_size(LogicalSize {
                width: 800,
                height: 600,
            })
            .build(&event_loop)
            .expect("Failed to create window");
        let surface = unsafe { instance.create_surface(&window) };
        Some((window, event_loop, surface))
    };

    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: window_stuff.as_ref().map(|(_, _, surface)| surface),
        })
        .await
        .expect("Unable to find a suitable graphics adapter");
    let info = adapter.get_info();
    println!(
        "Using {} {} ({})",
        match info.device_type {
            DeviceType::Other => "unclassified accelerator",
            DeviceType::IntegratedGpu => "integrated GPU",
            DeviceType::DiscreteGpu => "discrete GPU",
            DeviceType::VirtualGpu => "virtualized GPU",
            DeviceType::Cpu => "CPU",
        },
        info.name,
        match info.backend {
            Backend::Empty => "dummy backend",
            Backend::Vulkan => "Vulkan",
            Backend::Metal => "Metal",
            Backend::Dx12 => "DirectX 12",
            Backend::Dx11 => "DirectX 11",
            Backend::Gl => "OpenGL",
            Backend::BrowserWebGpu => "WebGPU",
        }
    );
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: Some("main device"),
                features: Features::default(),
                limits: Limits {
                    max_storage_buffers_per_shader_stage: 7,
                    ..Limits::default()
                },
            },
            None,
        )
        .await
        .expect("Failed to get device handle");

    let file = File::open(config_file).expect("Cannot open config file");
    let config = serde_yaml::from_reader(file).expect("Invalid config file");
    let simulation = Simulation::from_config(&device, config);

    if headless {
        run_headless(&device, &queue, simulation, checkpoint, steps)
    } else {
        let (window, event_loop, surface) = window_stuff.unwrap();
        run_headed(
            device, queue, adapter, surface, simulation, window, event_loop,
        )
    }
}

fn run_headed(
    device: Device,
    queue: Queue,
    adapter: Adapter,
    surface: Surface,
    simulation: Simulation,
    window: Window,
    event_loop: EventLoop<()>,
) -> ! {
    let visualization = Visualization::with_random_colors(&device, &adapter, &surface, simulation);
    visualization.run(device, queue, window, surface, event_loop)
}

fn run_headless(
    device: &Device,
    queue: &Queue,
    mut simulation: Simulation,
    checkpoint: Option<u64>,
    max_steps: Option<u64>,
) {
    let broken = Arc::new(AtomicBool::new(false));
    let b = broken.clone();
    ctrlc::set_handler(move || {
        b.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    let mut steps: u64 = 0;
    let mut steps_since_checkpoint: u64 = 0;
    let start = Instant::now();
    let mut last_checkpoint = start;

    loop {
        simulation.step(&device, &queue);
        steps += 1;
        steps_since_checkpoint += 1;
        if let Some(checkpoint) = checkpoint {
            if steps % checkpoint == 0 {
                let now = Instant::now();
                let tps = steps_since_checkpoint as f32 / (now - last_checkpoint).as_secs_f32();
                println!("Checkpoint {}. {} steps total. Running time: {:#?}. Average steps per second since last checkpoint: {} ({}x realtime)",
                    steps / checkpoint,
                    steps,
                    now - start,
                    tps as u32,
                    (tps / 60.0) as u32
                );
                last_checkpoint = now;
                steps_since_checkpoint = 0;
            }
        }
        if broken.load(Ordering::Relaxed)
            || max_steps
                .map(|max_steps| steps >= max_steps)
                .unwrap_or(false)
        {
            break;
        }
    }

    println!("Ran {} steps for {:#?}", steps, (Instant::now() - start));
    // TODO: saving
}
