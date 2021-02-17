// mod serialize;
mod simulation;
// mod visualization;

use std::{
    convert::TryInto,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

// use bincode::deserialize_from; TODO serialization
// use serialize::{RulesetSerialized, SimulationSerialized}; TODO serialization
use simulation::*;
use structopt::{clap::arg_enum, StructOpt};
use wgpu::*;
// use visualization::*;

#[derive(StructOpt)]
enum RulesetSource {
    Template { template: RulesetTemplateCLI },
    LoadRuleset { ruleset_path: PathBuf },
}

#[derive(StructOpt)]
enum SimulationSource {
    New {
        #[structopt(subcommand)]
        ruleset: RulesetSource,
        #[structopt(long)]
        points: u32,
        #[structopt(long)]
        wall_type: WallsCLI,
        #[structopt(long)]
        wall_dist: Option<f32>, // TODO is there a better way to do this?
    },
    Load {
        simulation_path: PathBuf,
    },
}

#[derive(StructOpt)]
struct HeadlessOptions {
    #[structopt(long)]
    checkpoint: Option<u64>,
    #[structopt(long)]
    steps: Option<u64>,
}

#[derive(StructOpt)]
/// Particle life simulator
struct RunSimulation {
    #[structopt(flatten)]
    simulation: SimulationSource,
    #[structopt(long)]
    headless: bool,
    #[structopt(flatten)]
    headless_options: HeadlessOptions,
}

arg_enum! {
    enum RulesetTemplateCLI {
        Cool,
        Diversity,
        Balanced,
        Chaos,
        Homogeneity,
        Quiescence,
    }
}

impl Into<RulesetTemplate> for RulesetTemplateCLI {
    fn into(self) -> RulesetTemplate {
        match self {
            RulesetTemplateCLI::Cool => COOL_TEMPLATE,
            RulesetTemplateCLI::Diversity => DIVERSITY_TEMPLATE,
            RulesetTemplateCLI::Balanced => BALANCED_TEMPLATE,
            RulesetTemplateCLI::Chaos => CHAOS_TEMPLATE,
            RulesetTemplateCLI::Homogeneity => HOMOGENEITY_TEMPLATE,
            RulesetTemplateCLI::Quiescence => QUIESCENCE_TEMPLATE,
        }
    }
}

arg_enum! {
    pub enum WallsCLI {
        None,
        Square,
        Wrapping,
    }
}

impl TryInto<Walls> for (WallsCLI, Option<f32>) {
    type Error = &'static str;
    fn try_into(self) -> Result<Walls, Self::Error> {
        match self {
            (WallsCLI::None, _) => Ok(Walls::None),
            (WallsCLI::Square, Some(f)) => Ok(Walls::Square(f)),
            (WallsCLI::Wrapping, Some(f)) => Ok(Walls::Wrapping(f)),
            _ => Err("Square and Wrapping walls require wall distance parameter."),
        }
    }
}

fn run_headed(simulation: Simulation) {
    unimplemented!()
    // let visualization = Visualization::with_random_colors(simulation);
    // let res = ContextBuilder::new("plife", "Taylor")
    //     .window_setup(WindowSetup {
    //         title: "plife visualization".to_owned(),
    //         samples: NumSamples::Four,
    //         vsync: true,
    //         icon: "".to_owned(),
    //         srgb: false,
    //     })
    //     .window_mode(WindowMode {
    //         width: 1200.0,
    //         height: 800.0,
    //         ..Default::default()
    //     })
    //     .modules(ModuleConf {
    //         gamepad: false,
    //         audio: false,
    //     })
    //     .build();
    // let (ctx, event_loop) = match res {
    //     Ok(tuple) => tuple,
    //     Err(e) => {
    //         eprintln!("Error initializing visualization: {}", e);
    //         std::process::exit(1);
    //     }
    // };

    // event::run(ctx, event_loop, visualization)
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
    println!("SAVING... PLEASE WAIT...");
    let save_start = Instant::now();
    let file_name: PathBuf = format!(
        "{}.bin",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    )
    .into();
    // bincode::serialize_into(
    //     std::fs::File::create(file_name).unwrap(),
    //     &Into::<SimulationSerialized>::into(simulation),
    // )
    // .unwrap(); TODO serialization
    println!("Saved in {:#?}", Instant::now() - save_start);
}

#[paw::main]
#[tokio::main]
async fn main(args: RunSimulation) {
    let RunSimulation {
        simulation: simulation_source,
        headless,
        headless_options,
    } = args;
    let instance = Instance::new(BackendBit::all());
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: None, // TODO visualization
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
                limits: Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to get device handle");

    let simulation = match simulation_source {
        SimulationSource::New {
            ruleset,
            points,
            wall_type,
            wall_dist,
        } => {
            let ruleset = match ruleset {
                RulesetSource::Template { template } => {
                    Into::<RulesetTemplate>::into(template).generate()
                }
                RulesetSource::LoadRuleset { ruleset_path } => {
                    // deserialize_from::<_, RulesetSerialized>(
                    //     std::fs::File::create(ruleset_path).unwrap(),
                    // )
                    // .unwrap()
                    // .into() TODO serialization
                    unimplemented!()
                }
            };
            Simulation::new(
                &device,
                points,
                ruleset,
                (wall_type, wall_dist).try_into().unwrap(),
            )
        }
        SimulationSource::Load { simulation_path } =>
        // deserialize_from::<_, SimulationSerialized>(
        //     std::fs::File::create(simulation_path).unwrap(),
        // )
        // .unwrap()
        // .into(), TODO serialization
        {
            unimplemented!()
        }
    };

    if headless {
        run_headless(
            &device,
            &queue,
            simulation,
            headless_options.checkpoint,
            headless_options.steps,
        )
    } else {
        run_headed(simulation)
    }
}
