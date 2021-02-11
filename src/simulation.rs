use std::unimplemented;

use arrayfire::*;
use num_complex::Complex;
use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

pub type Float = f32;
pub type Radius = Float;
pub type Attraction = Float;
pub type Friction = Float;
pub type PointType = u64;

pub struct Ruleset {
    pub num_point_types: PointType,
    pub min_r: Array<Radius>,
    pub max_r: Array<Radius>,
    pub attractions: Array<Attraction>,
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

        fn gen_2d_vec_uniform(n: PointType, min: Radius, max: Radius) -> Array<Radius> {
            let mut vec = Vec::with_capacity((n * n) as usize);
            for _ in 0..(n * n) {
                vec.push(thread_rng().gen_range(min..=max));
            }
            Array::new(&vec, Dim4::new(&[n, n, 1, 1]))
        }

        fn gen_2d_vec_normal(n: PointType, mean: Attraction, std_dev: Attraction) -> Array<Radius> {
            let dist = Normal::new(mean, std_dev).unwrap();
            let mut rng = thread_rng();
            let mut vec = Vec::with_capacity((n * n) as usize);
            for _ in 0..(n * n) {
                vec.push(dist.sample(&mut rng));
            }
            Array::new(&vec, Dim4::new(&[n, n, 1, 1]))
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

#[derive(Serialize, Deserialize)]
pub enum Walls {
    None,
    Square(Float),
    Wrapping(Float),
}

pub struct Simulation {
    pub positions: Array<Complex<f32>>,
    pub velocities: Array<Complex<f32>>,
    pub types: Array<PointType>,
    pub num_points: u64,
    pub ruleset: Ruleset,
    pub walls: Walls,
    pub cache_max_r: Array<Radius>, // TODO: make these private again
    pub cache_min_r: Array<Radius>,
    pub cache_attraction: Array<Attraction>,
}

impl Simulation {
    pub const R_SMOOTH: Float = 2.0;

    // utility
    fn generate_point_normal() -> Complex<f32> {
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, 5.0).unwrap();
        Complex::new(rng.sample(normal), rng.sample(normal))
    }

    fn generate_point_uniform(dist: Float) -> Complex<f32> {
        let mut rng = thread_rng();
        Complex::new(rng.gen_range(-dist..dist), rng.gen_range(-dist..dist))
    }

    pub fn new(num_points: u64, ruleset: Ruleset, walls: Walls) -> Self {
        let dims = Dim4::new(&[num_points, 1, 1, 1]);
        let mut points = Vec::with_capacity(num_points as usize);

        (0..num_points).for_each(|_| {
            let point = match walls {
                Walls::None => Simulation::generate_point_normal(),
                Walls::Square(dist) | Walls::Wrapping(dist) => {
                    Simulation::generate_point_uniform(dist)
                }
            };
            points.push(point);
        });

        let types = randu::<PointType>(dims) % constant(ruleset.num_point_types, dims);

        let idxr = || {
            let mut idx = Indexer::default();
            idx.set_index(&types, 0, None);
            idx.set_index(&types, 1, None);
            idx
        };

        Self {
            positions: Array::new(&points, dims),
            velocities: constant(Complex::new(0.0, 0.0), dims),
            num_points,
            walls,
            cache_max_r: index_gen(&ruleset.max_r, idxr()),
            cache_min_r: index_gen(&ruleset.min_r, idxr()),
            cache_attraction: index_gen(&ruleset.attractions, idxr()),
            types,
            ruleset,
        }
    }

    fn get_velocities(&self) -> Array<Complex<f32>> {
        let squared_dim = Dim4::new(&[self.num_points, self.num_points, 1, 1]);
        let p = tile(&self.positions, Dim4::new(&[1, self.num_points, 1, 1]));
        let q = tile(
            &transpose(&self.positions, false),
            Dim4::new(&[self.num_points, 1, 1, 1]),
        );
        let mut delta = q - p;
        if let Walls::Wrapping(wall_dist) = self.walls {
            let x_too_high = gt(&real(&delta), &wall_dist, false);
            let x_too_low = lt(&real(&delta), &-wall_dist, false);
            let y_too_high = gt(&imag(&delta), &wall_dist, false);
            let y_too_low = lt(&imag(&delta), &-wall_dist, false);

            delta -= x_too_high * wall_dist * Complex::new(2.0, 0.0);
            delta += x_too_low * wall_dist * Complex::new(2.0, 0.0);
            delta -= y_too_high * wall_dist * Complex::new(0.0, 2.0);
            delta += y_too_low * wall_dist * Complex::new(0.0, 2.0);
        }
        let dist = abs(&delta);
        let skip = or(
            &gt(&dist, &self.cache_max_r, false),
            &le(&dist, &constant(0.01f32, squared_dim), false),
            false,
        );
        let outside_minimum = gt(&dist, &self.cache_min_r, false);
        let numer = 2.0f32 * abs(&(&dist - 0.5f32 * (&self.cache_max_r + &self.cache_min_r)));
        let denom = &self.cache_max_r - &self.cache_min_r;
        let if_outside = &self.cache_attraction * (1.0f32 - numer / denom);
        let if_inside = Self::R_SMOOTH
            * &self.cache_min_r
            * (1.0f32 / (&self.cache_min_r + Self::R_SMOOTH) - 1.0f32 / (&dist + Self::R_SMOOTH));
        let force = &outside_minimum * if_outside + !&outside_minimum * if_inside;
        sum_nan(&(delta / dist * force * !&skip), 1, 0.0)
    }

    fn step_velocities(&mut self) {
        self.velocities += self.get_velocities();
    }

    pub fn step(&mut self) {
        self.step_velocities();

        self.positions = &self.positions + &self.velocities;
        self.velocities *= constant(
            1.0 - self.ruleset.friction,
            Dim4::new(&[self.num_points, 1, 1, 1]),
        );

        // Check for wall collisions
        let wall_dist = match self.walls {
            Walls::Wrapping(wall_dist) | Walls::Square(wall_dist) => wall_dist,
            Walls::None => return,
        };

        let x_too_low = lt(&real(&self.positions), &-wall_dist, false);
        let x_too_high = ge(&real(&self.positions), &wall_dist, false);
        let y_too_low = lt(&imag(&self.positions), &-wall_dist, false);
        let y_too_high = ge(&imag(&self.positions), &wall_dist, false);

        match self.walls {
            Walls::Wrapping(wall_dist) => {
                self.positions += &x_too_low * wall_dist * Complex::new(2.0, 0.0);
                self.positions -= &x_too_high * wall_dist * Complex::new(2.0, 0.0);
                self.positions += &y_too_low * wall_dist * Complex::new(0.0, 2.0);
                self.positions -= &y_too_high * wall_dist * Complex::new(0.0, 2.0);
            }
            Walls::Square(_) => {
                unimplemented!(); // TODO

                // ?????
                // let clamped_x_low =
                //     &self.positions * Complex::new(0.0f32, 1.0) + Complex::new(-wall_dist, 0.0);
                // self.positions = &x_too_low * &clamped_x_low + &!&x_too_low * &self.positions;
                // self.velocities *= Complex::new(-1.0, 1.0) * or(&x_too_low, &x_too_high, false);
                // self.velocities *= Complex::new(1.0, -1.0) * or(&y_too_low, &y_too_high, false);
            }
            Walls::None => {}
        }
    }
}
