use crate::simulation::Simulation;
use ggez::{
    event::{EventHandler, MouseButton},
    graphics::{self, Color, DrawMode, DrawParam, MeshBuilder, Text, Transform},
    input, Context, GameResult,
};

pub struct Visualization {
    pub simulation: Simulation,
    pub colors: Vec<Color>,
    pub ticks: u64,
    pub camera_offset: glam::Vec2, // TODO: use glam types everywhere
    pub zoom: f32,
}

impl Visualization {
    pub fn with_random_colors(simulation: Simulation) -> Self {
        let mut colors = Vec::with_capacity(simulation.ruleset.num_point_types);

        fn random_color() -> Color {
            Color::new(rand::random(), rand::random(), rand::random(), 1.0)
        }

        (0..simulation.ruleset.num_point_types).for_each(|_| colors.push(random_color()));
        Visualization {
            simulation,
            colors,
            ticks: 0,
            camera_offset: glam::Vec2::new(0.0, 0.0),
            zoom: 1.0,
        }
    }
}

impl EventHandler for Visualization {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        self.simulation.step();
        self.ticks += 1;

        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) {
        if y > 0.0 {
            self.zoom *= 2.0;
        } else if y < 0.0 {
            self.zoom /= 2.0;
        }
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, _x: f32, _y: f32, dx: f32, dy: f32) {
        if input::mouse::button_pressed(ctx, MouseButton::Left) {
            self.camera_offset += glam::Vec2::from((dx, dy)) / self.zoom;
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, graphics::Color::BLACK);

        let mut mesh_builder = MeshBuilder::new();

        for point in &self.simulation.points {
            mesh_builder.circle(
                DrawMode::fill(),
                [point.position.x, point.position.y],
                3.0,
                0.1,
                self.colors[point.point_type].clone(),
            )?;
        }

        let mesh = mesh_builder.build(ctx)?;

        let draw_params = DrawParam {
            trans: Transform::Values {
                dest: (self.camera_offset * self.zoom
                    + glam::Vec2::from(ggez::graphics::size(ctx)) / 2.0)
                    .into(),
                rotation: 0.0,
                scale: glam::Vec2::splat(self.zoom).into(),
                offset: glam::Vec2::new(0.0, 0.0).into(),
            },
            ..Default::default()
        };

        graphics::draw(ctx, &mesh, draw_params)?;

        let fps_text = Text::new(format!(
            "fps: {}\ntps: {}",
            ggez::timer::fps(ctx) as i32,
            (self.ticks as f32 / ggez::timer::time_since_start(ctx).as_secs_f32()) as i32
        ));

        graphics::draw::<_, graphics::DrawParam>(ctx, &fps_text, Default::default())?;

        graphics::present(ctx)
    }
}
