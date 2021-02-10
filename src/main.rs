mod simulation;
mod visualization;

use ggez::{
    conf::{ModuleConf, NumSamples, WindowMode, WindowSetup},
    event, ContextBuilder,
};
use simulation::*;
use structopt::StructOpt;
use visualization::*;

#[derive(StructOpt)]
enum RulesetTemplateCLI {
    Diversity,
}

impl Into<RulesetTemplate> for RulesetTemplateCLI {
    fn into(self) -> RulesetTemplate {
        match self {
            RulesetTemplateCLI::Diversity => DIVERSITY_TEMPLATE,
        }
    }
}

#[derive(StructOpt)]
/// Particle life simulator
enum CLIAction {
    /// Run and display a simulation
    Run(RulesetTemplateCLI),
}

#[paw::main]
fn main(args: CLIAction) {
    match args {
        CLIAction::Run(template) => {
            let ruleset = Into::<RulesetTemplate>::into(template).generate();
            let simulation = Simulation::new(2000, ruleset);

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
