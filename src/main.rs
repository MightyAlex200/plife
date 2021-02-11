mod serialize;
mod simulation;
mod visualization;

use std::{
    convert::TryInto,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use bincode::deserialize_from;
use ggez::{
    conf::{ModuleConf, NumSamples, WindowMode, WindowSetup},
    event, ContextBuilder,
};
use serialize::{RulesetSerialized, SimulationSerialized};
use simulation::*;
use structopt::{clap::arg_enum, StructOpt};
use visualization::*;

#[derive(StructOpt)]
/// Particle life simulator
enum CLIAction {
    /// Run and display a simulation
    Run {
        #[structopt(
            long,
            required_unless("ruleset-template"),
            conflicts_with("ruleset-template")
        )]
        ruleset: Option<PathBuf>,
        #[structopt(long, required_unless("ruleset"))]
        ruleset_template: Option<RulesetTemplateCLI>,
        #[structopt(long)]
        walls: WallsCLI,
        #[structopt(long, default_value = "1000")]
        points: u64,
        #[structopt(long, required_ifs(&[("walls", "square"), ("walls", "wrapping")]))]
        wall_dist: Option<Float>,
        #[structopt(long)]
        headless: bool,
        #[structopt(long, required_if("headless", "true"))]
        checkpoint: Option<u64>,
        #[structopt(long)]
        steps: Option<u64>,
    },
}

arg_enum! {
    enum RulesetTemplateCLI {
        Diversity,
    }
}

impl Into<RulesetTemplate> for RulesetTemplateCLI {
    fn into(self) -> RulesetTemplate {
        match self {
            RulesetTemplateCLI::Diversity => DIVERSITY_TEMPLATE,
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

impl TryInto<Walls> for (WallsCLI, Option<Float>) {
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
    let visualization = Visualization::with_random_colors(simulation);
    let res = ContextBuilder::new("plife", "Taylor")
        .window_setup(WindowSetup {
            title: "plife visualization".to_owned(),
            samples: NumSamples::Four,
            vsync: true,
            icon: "".to_owned(),
            srgb: false,
        })
        .window_mode(WindowMode {
            width: 1200.0,
            height: 800.0,
            ..Default::default()
        })
        .modules(ModuleConf {
            gamepad: false,
            audio: false,
        })
        .build();
    let (ctx, event_loop) = match res {
        Ok(tuple) => tuple,
        Err(e) => {
            eprintln!("Error initializing visualization: {}", e);
            std::process::exit(1);
        }
    };

    event::run(ctx, event_loop, visualization)
}

fn run_headless(mut simulation: Simulation, checkpoint: Option<u64>, max_steps: Option<u64>) {
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
        simulation.step();
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
    bincode::serialize_into(
        std::fs::File::create(file_name).unwrap(),
        &Into::<SimulationSerialized>::into(simulation),
    )
    .unwrap();
    println!("Saved in {:#?}", Instant::now() - save_start);
}

#[paw::main]
fn main(args: CLIAction) {
    match args {
        CLIAction::Run {
            ruleset,
            walls,
            wall_dist,
            points,
            ruleset_template,
            headless,
            checkpoint,
            steps,
        } => {
            let ruleset = match (ruleset, ruleset_template) {
                (Some(path), _) => {
                    deserialize_from::<_, RulesetSerialized>(std::fs::File::create(path).unwrap())
                        .unwrap()
                        .into()
                }
                (None, Some(template)) => Into::<RulesetTemplate>::into(template).generate(),
                (None, None) => unreachable!(),
            };
            let simulation =
                Simulation::new(points, ruleset, (walls, wall_dist).try_into().unwrap());

            if headless {
                run_headless(simulation, checkpoint, steps)
            } else {
                run_headed(simulation)
            }
        }
    }
}
