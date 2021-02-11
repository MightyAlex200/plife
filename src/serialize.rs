use arrayfire::{Array, Dim4};
use num_complex::Complex;
use serde::{Deserialize, Serialize};

use crate::simulation::*;

#[derive(Serialize, Deserialize)]
pub struct Dim4Serialized(u64, u64, u64, u64);

#[derive(Serialize, Deserialize)]
pub struct RulesetSerialized {
    pub num_point_types: PointType,
    pub min_r_vals: Vec<Radius>,
    pub min_r_dims: Dim4Serialized,
    pub max_r_vals: Vec<Radius>,
    pub max_r_dims: Dim4Serialized,
    pub attractions_vals: Vec<Attraction>,
    pub attractions_dims: Dim4Serialized,
    pub friction: Friction,
}

#[derive(Serialize, Deserialize)]
pub struct ComplexSerialized<T> {
    real: T,
    imag: T,
}

#[derive(Serialize, Deserialize)]
pub struct SimulationSerialized {
    pub positions_vals: Vec<ComplexSerialized<f32>>,
    pub positions_dims: Dim4Serialized,
    pub velocities_vals: Vec<ComplexSerialized<f32>>,
    pub velocities_dims: Dim4Serialized,
    pub types_vals: Vec<PointType>,
    pub types_dims: Dim4Serialized,
    pub num_points: u64,
    pub ruleset: RulesetSerialized,
    pub walls: Walls,
    // TODO: do not store these!
    pub cache_max_r_vals: Vec<Radius>,
    pub cache_max_r_dims: Dim4Serialized,
    pub cache_min_r_vals: Vec<Radius>,
    pub cache_min_r_dims: Dim4Serialized,
    pub cache_attraction_vals: Vec<Attraction>,
    pub cache_attraction_dims: Dim4Serialized,
}

impl Into<Dim4> for Dim4Serialized {
    fn into(self) -> Dim4 {
        let Dim4Serialized(x, y, z, w) = self;
        Dim4::new(&[x, y, z, w])
    }
}

impl Into<Ruleset> for RulesetSerialized {
    fn into(self) -> Ruleset {
        Ruleset {
            num_point_types: self.num_point_types,
            min_r: Array::new(&self.min_r_vals, self.min_r_dims.into()),
            max_r: Array::new(&self.max_r_vals, self.max_r_dims.into()),
            attractions: Array::new(&self.attractions_vals, self.attractions_dims.into()),
            friction: self.friction,
        }
    }
}

impl Into<Simulation> for SimulationSerialized {
    fn into(self) -> Simulation {
        Simulation {
            positions: Array::new(
                &self
                    .positions_vals
                    .into_iter()
                    .map(Into::<Complex<f32>>::into)
                    .collect::<Vec<_>>(),
                self.positions_dims.into(),
            ),
            velocities: Array::new(
                &self
                    .velocities_vals
                    .into_iter()
                    .map(Into::<Complex<f32>>::into)
                    .collect::<Vec<_>>(),
                self.velocities_dims.into(),
            ),
            types: Array::new(&self.types_vals, self.types_dims.into()),
            num_points: self.num_points,
            ruleset: self.ruleset.into(),
            walls: self.walls,
            cache_max_r: Array::new(&self.cache_max_r_vals, self.cache_max_r_dims.into()),
            cache_min_r: Array::new(&self.cache_min_r_vals, self.cache_min_r_dims.into()),
            cache_attraction: Array::new(
                &self.cache_attraction_vals,
                self.cache_attraction_dims.into(),
            ),
        }
    }
}

impl<T> Into<Complex<T>> for ComplexSerialized<T> {
    fn into(self) -> Complex<T> {
        Complex::new(self.real, self.imag)
    }
}

// other way

impl Into<SimulationSerialized> for Simulation {
    fn into(self) -> SimulationSerialized {
        let mut positions_vals: Vec<Complex<f32>> =
            vec![Complex::new(0.0, 0.0); self.positions.elements()];
        let positions_dims = self.positions.dims().into();
        self.positions.host(&mut positions_vals);
        let positions_vals = positions_vals
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let mut velocities_vals: Vec<Complex<f32>> =
            vec![Complex::new(0.0, 0.0); self.velocities.elements()];
        let velocities_dims = self.velocities.dims().into();
        self.velocities.host(&mut velocities_vals);
        let velocities_vals = velocities_vals
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let mut types_vals = vec![0; self.types.elements()];
        let types_dims = self.types.dims().into();
        self.types.host(&mut types_vals);
        let mut cache_max_r_vals = vec![0.0; self.cache_max_r.elements()];
        let cache_max_r_dims = self.cache_max_r.dims().into();
        self.cache_max_r.host(&mut cache_max_r_vals);
        let mut cache_min_r_vals = vec![0.0; self.cache_min_r.elements()];
        let cache_min_r_dims = self.cache_min_r.dims().into();
        self.cache_min_r.host(&mut cache_min_r_vals);
        let mut cache_attraction_vals = vec![0.0; self.cache_attraction.elements()];
        let cache_attraction_dims = self.cache_attraction.dims().into();
        self.cache_attraction.host(&mut cache_attraction_vals);
        SimulationSerialized {
            positions_vals,
            positions_dims,
            velocities_vals,
            velocities_dims,
            types_vals,
            types_dims,
            num_points: self.num_points,
            ruleset: self.ruleset.into(),
            walls: self.walls.into(),
            cache_max_r_vals,
            cache_max_r_dims,
            cache_min_r_vals,
            cache_min_r_dims,
            cache_attraction_vals,
            cache_attraction_dims,
        }
    }
}

impl Into<RulesetSerialized> for Ruleset {
    fn into(self) -> RulesetSerialized {
        let mut min_r_vals = vec![0.0; self.min_r.elements()];
        let min_r_dims = self.min_r.dims().into();
        self.min_r.host(&mut min_r_vals);
        let mut max_r_vals = vec![0.; self.max_r.elements()];
        let max_r_dims = self.max_r.dims().into();
        self.max_r.host(&mut max_r_vals);
        let mut attractions_vals = vec![0.0; self.attractions.elements()];
        let attractions_dims = self.attractions.dims().into();
        self.attractions.host(&mut attractions_vals);
        RulesetSerialized {
            num_point_types: self.num_point_types,
            min_r_vals,
            min_r_dims,
            max_r_vals,
            max_r_dims,
            attractions_vals,
            attractions_dims,
            friction: self.friction,
        }
    }
}

impl Into<Dim4Serialized> for Dim4 {
    fn into(self) -> Dim4Serialized {
        let vals = self.get();
        Dim4Serialized(vals[0], vals[1], vals[2], vals[3])
    }
}

impl<T> Into<ComplexSerialized<T>> for Complex<T> {
    fn into(self) -> ComplexSerialized<T> {
        ComplexSerialized {
            real: self.re,
            imag: self.im,
        }
    }
}
