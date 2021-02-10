use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;

pub type Float = f32;
pub type Radius = Float;
pub type Attraction = Float;
pub type Friction = Float;
pub type PointType = usize;

pub type Vec2 = glam::Vec2;

pub struct Point {
    pub position: Vec2,
    pub point_type: PointType,
    pub velocity: Vec2,
}

impl Point {
    pub fn generate_normal(point_types: PointType) -> Self {
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, 5.0).unwrap();
        Self {
            position: Vec2::new(rng.sample(normal), rng.sample(normal)),
            point_type: rng.gen_range(0..point_types),
            velocity: Vec2::zero(),
        }
    }

    pub fn generate_uniform(point_types: PointType, dist: Float) -> Self {
        let mut rng = thread_rng();
        Self {
            position: Vec2::new(rng.gen_range(-dist..dist), rng.gen_range(-dist..dist)),
            point_type: rng.gen_range(0..point_types),
            velocity: Vec2::zero(),
        }
    }
}

#[derive(Debug)]
pub struct Ruleset {
    pub num_point_types: PointType,
    pub min_r: Vec<Vec<Radius>>,
    pub max_r: Vec<Vec<Radius>>,
    pub attractions: Vec<Vec<Attraction>>,
    pub friction: Friction,
}

/// Stores values for randomly generating [Ruleset]s
#[derive(Debug)]
pub struct RulesetTemplate {
    pub min_types: PointType,
    pub max_types: PointType,
    pub min_friction: Friction,
    pub max_friction: Friction,
    pub min_r_lower: Radius,
    pub min_r_upper: Radius,
    pub max_r_lower: Radius,
    pub max_r_upper: Radius,
    pub attractions_mean: Attraction,
    pub attractions_std: Attraction,
}

impl RulesetTemplate {
    pub fn generate(&self) -> Ruleset {
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

pub const DIVERSITY_TEMPLATE: RulesetTemplate = RulesetTemplate {
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

pub enum Walls {
    None,
    Square(Float),
    Wrapping(Float),
}

pub struct Simulation {
    pub points: Vec<Point>,
    pub ruleset: Ruleset,
    pub walls: Walls,
}

impl Simulation {
    pub const R_SMOOTH: Float = 2.0;

    pub fn new(num_points: usize, ruleset: Ruleset, walls: Walls) -> Self {
        let mut points = Vec::with_capacity(num_points);
        (0..num_points).for_each(|_| {
            let point = match walls {
                Walls::None => Point::generate_normal(ruleset.num_point_types),
                Walls::Square(dist) | Walls::Wrapping(dist) => {
                    Point::generate_uniform(ruleset.num_point_types, dist)
                }
            };
            points.push(point);
        });
        Self {
            points,
            ruleset,
            walls,
        }
    }

    fn get_velocity(&self, p: &Point) -> Vec2 {
        self.points
            .par_iter()
            .map(|q| {
                let mut delta = q.position - p.position;

                if let Walls::Wrapping(wall_dist) = self.walls {
                    if delta.x > wall_dist * 0.5 {
                        delta.x -= wall_dist;
                    } else if delta.x < -wall_dist * 0.5 {
                        delta.x += wall_dist;
                    }

                    if delta.y > wall_dist * 0.5 {
                        delta.y -= wall_dist;
                    } else if delta.y < -delta.y * 0.5 {
                        delta.y += wall_dist;
                    }
                }

                // Get distance squared
                let r2 = delta.x * delta.x + delta.y * delta.y;
                let min_r = self.ruleset.min_r[p.point_type][q.point_type];
                let max_r = self.ruleset.max_r[p.point_type][q.point_type];

                if r2 > max_r * max_r || r2 < 0.01 {
                    return Vec2::zero();
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
            .collect::<Vec<_>>()
            .iter()
            .sum::<Vec2>()
    }

    fn step_velocities(&mut self) {
        let velocities = self
            .points
            .par_iter()
            .map(|p| self.get_velocity(p))
            .collect::<Vec<Vec2>>();

        self.points.par_iter_mut().enumerate().for_each(|(i, p)| {
            p.velocity = velocities[i];
        });
    }

    pub fn step(&mut self) {
        self.step_velocities();

        // Update position
        for p in self.points.iter_mut() {
            // Update position and velocity
            p.position += p.velocity;
            p.velocity *= 1.0 - self.ruleset.friction;

            // Check for wall collisions
            match self.walls {
                Walls::Wrapping(wall_dist) => {
                    if p.position.x < -wall_dist {
                        p.position.x += wall_dist * 2.0;
                    } else if p.position.x >= wall_dist {
                        p.position.x -= wall_dist * 2.0;
                    }

                    if p.position.y < -wall_dist {
                        p.position.y += wall_dist * 2.0;
                    } else if p.position.y >= wall_dist {
                        p.position.y -= wall_dist * 2.0;
                    }
                }
                Walls::Square(wall_dist) => {
                    if p.position.x < -wall_dist {
                        p.velocity.x = -p.velocity.x;
                        p.position.x = -wall_dist;
                    } else if p.position.x >= wall_dist {
                        p.velocity.x = -p.velocity.x;
                        p.position.x = wall_dist;
                    }

                    if p.position.y < -wall_dist {
                        p.velocity.y = -p.velocity.y;
                        p.position.y = -wall_dist;
                    } else if p.position.y >= wall_dist {
                        p.velocity.y = -p.velocity.y;
                        p.position.y = wall_dist;
                    }
                }
                Walls::None => {}
            }
        }
    }
}
