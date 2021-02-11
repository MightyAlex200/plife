mod simulation;
mod visualization;

use std::convert::TryInto;

use ggez::{
    conf::{ModuleConf, NumSamples, WindowMode, WindowSetup},
    event, ContextBuilder,
};
use simulation::*;
use structopt::{clap::arg_enum, StructOpt};
use visualization::*;

#[derive(StructOpt)]
/// Particle life simulator
enum CLIAction {
    /// Run and display a simulation
    Run {
        #[structopt(long)]
        ruleset: RulesetTemplateCLI,
        #[structopt(long)]
        walls: WallsCLI,
        #[structopt(long, default_value = "1000")]
        points: u64,
        #[structopt(long, required_ifs(&[("walls", "square"), ("walls", "wrapping")]))]
        wall_dist: Option<Float>,
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

#[paw::main]
fn main(args: CLIAction) {
    match args {
        CLIAction::Run {
            ruleset,
            walls,
            wall_dist,
            points,
        } => {
            let ruleset = Into::<RulesetTemplate>::into(ruleset).generate();
            let simulation =
                Simulation::new(points, ruleset, (walls, wall_dist).try_into().unwrap());

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
    }
}
