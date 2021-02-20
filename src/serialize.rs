use rand::{thread_rng, Rng};
use rand_distr::{
    num_traits::{NumCast, ToPrimitive},
    Normal,
};
use serde::Deserialize;

use crate::simulation::{Ruleset, Walls};

#[derive(Deserialize)]
pub struct Config {
    pub ruleset: RulesetConfig,
    pub walls: WallsConfig,
    pub points: PointsConfig,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Distribution<T> {
    Const(T),
    Uniform { min: T, max: T },
    Normal { mean: T, std: T },
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum RulesetConfig {
    Procedural(RulesetGenerationConfig),
    Precise {
        types: Vec<TypeRuleset>,
        friction: Distribution<f32>,
    },
}

#[derive(Deserialize)]
pub struct RulesetGenerationConfig {
    pub types: Distribution<u32>,
    pub attractions: Distribution<f32>,
    pub min_r: Distribution<f32>,
    pub max_r: Distribution<f32>,
    pub friction: Distribution<f32>,
}

#[derive(Deserialize)]
pub struct TypeRuleset {
    pub attractions: Vec<Distribution<f32>>,
    pub min_r: Vec<Distribution<f32>>,
    pub max_r: Vec<Distribution<f32>>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WallsConfig {
    None,
    Wrapping { dist: Distribution<f32> },
    Square { dist: Distribution<f32> },
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PointsConfig {
    Simple(Distribution<u32>),
    Complex(Vec<PointSpawnConfig>),
}

#[derive(Deserialize)]
pub struct PointSpawnConfig {
    pub num: Distribution<u32>,
    pub x: Distribution<f32>,
    pub y: Distribution<f32>,
}

impl Config {
    pub fn sample(self) -> (Ruleset, Walls, Vec<(f32, f32)>) {
        let ruleset = self.ruleset.sample();
        let walls = self.walls.sample();
        let points = self.points.sample(&walls);
        (ruleset, walls, points)
    }
}

macro_rules! typeruleset_map {
    ($types:expr, $prop:ident) => {
        $types
            .iter()
            .map(|ruleset| {
                ruleset
                    .$prop
                    .iter()
                    .map(|dist| dist.clone().sample())
                    .collect::<Vec<f32>>()
            })
            .collect::<Vec<Vec<f32>>>()
    };
}

impl RulesetConfig {
    fn sample(self) -> Ruleset {
        match self {
            RulesetConfig::Procedural(gen_rules) => gen_rules.sample(),
            RulesetConfig::Precise { types, friction } => Ruleset {
                num_point_types: types.len() as u32,
                min_r: typeruleset_map!(types, min_r),
                max_r: typeruleset_map!(types, max_r),
                attractions: typeruleset_map!(types, attractions),
                friction: friction.sample(),
            },
        }
    }
}

impl RulesetGenerationConfig {
    fn sample(self) -> Ruleset {
        fn sample_per_pair(num_point_types: u32, dist: Distribution<f32>) -> Vec<Vec<f32>> {
            let mut vec1 = Vec::with_capacity(num_point_types as usize);
            for _ in 0..num_point_types {
                let mut vec2 = Vec::with_capacity(num_point_types as usize);
                for _ in 0..num_point_types {
                    vec2.push(dist.clone().sample());
                }
                vec1.push(vec2);
            }
            vec1
        }

        let num_point_types = self.types.sample();
        Ruleset {
            num_point_types,
            min_r: sample_per_pair(num_point_types, self.min_r),
            max_r: sample_per_pair(num_point_types, self.max_r),
            attractions: sample_per_pair(num_point_types, self.attractions),
            friction: self.friction.sample(),
        }
    }
}

impl WallsConfig {
    fn sample(self) -> Walls {
        match self {
            WallsConfig::None => Walls::None,
            WallsConfig::Wrapping { dist } => Walls::Wrapping(dist.sample()),
            WallsConfig::Square { dist } => Walls::Square(dist.sample()),
        }
    }
}

impl PointsConfig {
    fn sample(self, walls: &Walls) -> Vec<(f32, f32)> {
        match self {
            PointsConfig::Simple(dist) => {
                let distribution = match walls {
                    Walls::None => Distribution::Normal {
                        mean: 0.0,
                        std: 5.0,
                    },
                    Walls::Square(dist) | Walls::Wrapping(dist) => Distribution::Uniform {
                        min: -dist,
                        max: *dist,
                    },
                };
                let num_points = dist.sample();
                let mut vec = Vec::with_capacity(num_points as usize);
                for _ in 0..num_points {
                    let x = distribution.clone().sample();
                    let y = distribution.clone().sample();
                    vec.push((x, y));
                }
                vec
            }
            PointsConfig::Complex(spawns) => spawns
                .into_iter()
                .map(|spawn| {
                    let num = spawn.num.sample();
                    let mut vec = Vec::with_capacity(num as usize);
                    for _ in 0..num {
                        let x = spawn.x.clone().sample();
                        let y = spawn.y.clone().sample();
                        vec.push((x, y));
                    }
                    vec
                })
                .flatten()
                .collect::<Vec<(f32, f32)>>(),
        }
    }
}

impl<T> Distribution<T>
where
    T: ToPrimitive + NumCast,
{
    fn sample(self) -> T {
        match self {
            Distribution::Const(t) => t,
            Distribution::Uniform { min, max } => {
                let min: f64 = NumCast::from(min).unwrap();
                let max: f64 = NumCast::from(max).unwrap();
                NumCast::from(thread_rng().gen_range(min..max)).unwrap()
            }
            Distribution::Normal { mean, std } => {
                let mean: f64 = NumCast::from(mean).unwrap();
                let std: f64 = NumCast::from(std).unwrap();
                let normal = Normal::new(mean, std).unwrap();
                NumCast::from(thread_rng().sample(normal)).unwrap()
            }
        }
    }
}
