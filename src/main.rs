use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};
use structopt::StructOpt;

// use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

type Float = f32;
type Radius = Float;
type Attraction = Float;
type Friction = Float;
type PointType = usize;

type Vec2 = nalgebra::Vector2<Float>;

struct Point {
    position: Vec2,
    point_type: PointType,
    velocity: Vec2,
}

impl Point {
    fn generate(point_types: PointType) -> Self {
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, 2.0).unwrap();
        Self {
            position: Vec2::new(rng.sample(normal), rng.sample(normal)),
            point_type: rng.gen_range(0..point_types),
            velocity: Vec2::zeros(),
        }
    }
}

#[derive(Debug)]
struct Ruleset {
    num_point_types: PointType,
    min_r: Vec<Vec<Radius>>,
    max_r: Vec<Vec<Radius>>,
    attractions: Vec<Vec<Attraction>>,
    friction: Friction,
}

/// Stores values for randomly generating [Ruleset]s
#[derive(Debug)]
struct RulesetTemplate {
    min_types: PointType,
    max_types: PointType,
    min_friction: Friction,
    max_friction: Friction,
    min_r_lower: Radius,
    min_r_upper: Radius,
    max_r_lower: Radius,
    max_r_upper: Radius,
    attractions_mean: Attraction,
    attractions_std: Attraction,
}

impl RulesetTemplate {
    fn generate(&self) -> Ruleset {
        let num_point_types = thread_rng().gen_range(self.min_types..=self.max_types);

        // un-idiomatic but I'm not smart :(

        fn gen_2d_vec_uniform(n: PointType, min: Radius, max: Radius) -> Vec<Vec<Radius>> {
            let mut vec1 = Vec::with_capacity(n);
            for _ in 0..n {
                let mut vec2 = Vec::with_capacity(n);
                for _ in 0..n {
                    vec2.push(thread_rng().gen_range(min..=max));
                }
                vec1.push(vec2);
            }
            vec1
        }

        fn gen_2d_vec_normal(
            n: PointType,
            mean: Attraction,
            std_dev: Attraction,
        ) -> Vec<Vec<Radius>> {
            let dist = Normal::new(mean, std_dev).unwrap();
            let mut rng = thread_rng();
            let mut vec1 = Vec::with_capacity(n);
            for _ in 0..n {
                let mut vec2 = Vec::with_capacity(n);
                for _ in 0..n {
                    vec2.push(dist.sample(&mut rng));
                }
                vec1.push(vec2);
            }
            vec1
        }

        Ruleset {
            num_point_types,
            min_r: gen_2d_vec_uniform(num_point_types, self.min_r_lower, self.min_r_upper),
            max_r: gen_2d_vec_uniform(num_point_types, self.max_r_lower, self.max_r_upper),
            attractions: gen_2d_vec_normal(
                num_point_types,
                self.attractions_mean,
                self.attractions_std,
            ),
            friction: thread_rng().gen_range(self.min_friction..=self.max_friction),
        }
    }
}

const DIVERSITY_TEMPLATE: RulesetTemplate = RulesetTemplate {
    min_types: 12,
    max_types: 12,
    attractions_mean: -0.01,
    attractions_std: 0.04,
    min_r_lower: 0.0,
    min_r_upper: 20.0,
    max_r_upper: 60.0,
    max_r_lower: 10.0,
    max_friction: 0.05,
    min_friction: 0.05,
};

struct Simulation {
    points: Vec<Point>,
    ruleset: Ruleset,
}

impl Simulation {
    const R_SMOOTH: Float = 2.0;

    fn new(num_points: usize, ruleset: Ruleset) -> Self {
        let mut points = Vec::with_capacity(num_points);
        (0..num_points).for_each(|_| {
            points.push(Point::generate(ruleset.num_point_types));
        });
        Self { points, ruleset }
    }

    fn get_velocity(&self, p: &Point) -> Vec2 {
        self.points
            .iter()
            .map(|q| {
                // _ type to silence rust-analysis from making huuuuge type!
                let mut delta: _ = q.position - p.position;

                //   if (this.wrap) {
                //     if (dx > this.width * 0.5) {
                //       dx -= this.width;
                //     } else if (dx < -this.width * 0.5) {
                //       dx += this.width;
                //     }

                //     if (dy > this.height * 0.5) {
                //       dy -= this.height;
                //     } else if (dy < -this.height * 0.5) {
                //       dy += this.height;
                //     }
                //   }

                // Get distance squared
                let r2 = delta.x * delta.x + delta.y * delta.y;
                let min_r = self.ruleset.min_r[p.point_type][q.point_type];
                let max_r = self.ruleset.max_r[p.point_type][q.point_type];

                if r2 > max_r * max_r || r2 < 0.01 {
                    return Vec2::zeros();
                }

                // Normalize displacement
                let r = r2.sqrt();
                delta /= r;

                // Calculate force
                let f: Float = if r > min_r {
                    // if (this.flatForce) {
                    //   f = this.types.getAttract(p.type, q.type);
                    // } else {
                    let numer = 2.0 * Float::abs(r - 0.5 * (max_r + min_r));
                    let denom = max_r - min_r;
                    self.ruleset.attractions[p.point_type][q.point_type] * (1.0 - numer / denom)
                // }
                } else {
                    Self::R_SMOOTH
                        * min_r
                        * (1.0 / (min_r + Self::R_SMOOTH) - 1.0 / (r + Self::R_SMOOTH))
                };

                delta * f
            })
            .sum::<Vec2>()
    }

    fn step_velocities(&mut self) {
        let velocities = self
            .points
            .iter()
            .map(|p| self.get_velocity(p))
            .collect::<Vec<Vec2>>();

        for (i, p) in self.points.iter_mut().enumerate() {
            p.velocity = velocities[i];
        }
    }

    fn step(&mut self) {
        self.step_velocities();

        // Update position
        for p in self.points.iter_mut() {
            // Update position and velocity
            p.position += p.velocity;
            p.velocity *= 1.0 - self.ruleset.friction;

            // // Check for wall collisions
            // if (this.wrap) {
            //   if (p.x < 0) {
            //     p.x += this.width;
            //   } else if (p.x >= this.width) {
            //     p.x -= this.width;
            //   }

            //   if (p.y < 0) {
            //     p.y += this.height;
            //   } else if (p.y >= this.height) {
            //     p.y -= this.height;
            //   }
            // } else {
            //   if (p.x < DIAMETER) {
            //     p.vx = -p.vx;
            //     p.x = DIAMETER;
            //   } else if (p.x >= this.width - DIAMETER) {
            //     p.vx = -p.vx;
            //     p.x = this.width - DIAMETER;
            //   }

            //   if (p.y < DIAMETER) {
            //     p.vy = -p.vy;
            //     p.y = DIAMETER;
            //   } else if (p.y >= this.height - DIAMETER) {
            //     p.vy = -p.vy;
            //     p.y = this.height - DIAMETER;
            //   }
            // }
        }
    }
}

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
            let mut simulation = Simulation::new(1000, ruleset);
            let mut iters: i64 = 0;
            loop {
                simulation.step();
                iters += 1;
                println!("{}", iters);
            }
        }
    }
}
