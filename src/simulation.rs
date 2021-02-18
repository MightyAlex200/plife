use std::{
    io::{Cursor, Write},
    num::NonZeroU64,
};

use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};
// use serde::{Deserialize, Serialize}; TODO serialization
use wgpu::*;

pub type Radius = f32;
pub type Attraction = f32;
pub type Friction = f32;
pub type PointType = u32;

const WORKGROUP_SIZE: u32 = 256;

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
            let mut vec1 = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let mut vec2 = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    vec2.push(thread_rng().gen_range(min..=max));
                }
                vec1.push(vec2)
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
            let mut vec1 = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let mut vec2 = Vec::with_capacity(n as usize);
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

pub const COOL_TEMPLATE: RulesetTemplate = RulesetTemplate {
    min_types: 12,
    max_types: 12,
    attractions_mean: -0.01,
    attractions_std: 0.04,
    min_r_lower: 0.0,
    min_r_upper: 20.0,
    max_r_upper: 500.0,
    max_r_lower: 10.0,
    max_friction: 0.05,
    min_friction: 0.05,
};
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
pub const BALANCED_TEMPLATE: RulesetTemplate = RulesetTemplate {
    min_types: 9,
    max_types: 9,
    attractions_mean: -0.02,
    attractions_std: 0.06,
    min_r_lower: 0.0,
    min_r_upper: 20.0,
    max_r_lower: 20.0,
    max_r_upper: 70.0,
    min_friction: 0.05,
    max_friction: 0.05,
};
pub const CHAOS_TEMPLATE: RulesetTemplate = RulesetTemplate {
    min_types: 6,
    max_types: 6,
    attractions_mean: 0.02,
    attractions_std: 0.04,
    min_r_lower: 0.0,
    min_r_upper: 30.0,
    max_r_lower: 30.0,
    max_r_upper: 100.0,
    min_friction: 0.01,
    max_friction: 0.01,
};
pub const HOMOGENEITY_TEMPLATE: RulesetTemplate = RulesetTemplate {
    min_types: 4,
    max_types: 4,
    attractions_mean: 0.0,
    attractions_std: 0.04,
    min_r_lower: 10.0,
    min_r_upper: 10.0,
    max_r_lower: 10.0,
    max_r_upper: 80.0,
    min_friction: 0.05,
    max_friction: 0.05,
};
pub const QUIESCENCE_TEMPLATE: RulesetTemplate = RulesetTemplate {
    min_types: 6,
    max_types: 6,
    attractions_mean: -0.02,
    attractions_std: 0.1,
    min_r_lower: 10.0,
    min_r_upper: 20.0,
    max_r_lower: 20.0,
    max_r_upper: 60.0,
    min_friction: 0.2,
    max_friction: 0.2,
};

// #[derive(Serialize, Deserialize)] TODO serialization
pub enum Walls {
    None,
    Square(f32),
    Wrapping(f32),
}

pub struct Simulation {
    pub num_points: u32,
    pub ruleset: Ruleset,
    pub walls: Walls,
    pub positions: Buffer,
    pub globals: Buffer,
    positions_old: Buffer,
    velocities: Buffer,
    types: Buffer,
    cache_max_r: Buffer,
    cache_min_r: Buffer,
    cache_attraction: Buffer,
    step_buffer: Option<CommandBuffer>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: ComputePipeline,
    velocities_old: Buffer,
}

impl Simulation {
    pub const R_SMOOTH: f32 = 2.0;

    // utility
    fn generate_point_normal() -> (f32, f32) {
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, 5.0).unwrap();
        (rng.sample(normal), rng.sample(normal))
    }

    fn generate_point_uniform(dist: f32) -> (f32, f32) {
        let mut rng = thread_rng();
        (rng.gen_range(-dist..dist), rng.gen_range(-dist..dist))
    }

    pub fn new(device: &Device, num_points: u32, ruleset: Ruleset, walls: Walls) -> Self {
        // Buffers
        fn create_buffer(
            device: &Device,
            label: Option<&'static str>,
            size: u64,
            usage: BufferUsage,
            map_function: impl FnOnce(&mut Buffer),
        ) -> Buffer {
            let mut buf = device.create_buffer(&BufferDescriptor {
                label,
                size,
                usage,
                mapped_at_creation: true,
            });
            map_function(&mut buf);
            buf.unmap();
            buf
        }

        let positions = create_buffer(
            &device,
            Some("positions"),
            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
            BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            |positions: &mut Buffer| {
                let slice = positions.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                for _ in 0..num_points {
                    let point = Simulation::generate_point_uniform(500.0); // TODO Walls
                    cursor.write_all(&point.0.to_le_bytes()).unwrap();
                    cursor.write_all(&point.1.to_le_bytes()).unwrap();
                }
            },
        );

        let positions_old = create_buffer(
            &device,
            Some("positions_old"),
            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
            BufferUsage::STORAGE | BufferUsage::COPY_DST,
            |_| {},
        );

        let velocities = create_buffer(
            &device,
            Some("velocities"),
            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
            BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            |velocities| {
                let slice = velocities.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                for _ in 0..num_points {
                    cursor.write_all(&0.0f32.to_le_bytes()).unwrap();
                    cursor.write_all(&0.0f32.to_le_bytes()).unwrap();
                }
            },
        );

        let velocities_old = create_buffer(
            &device,
            Some("velocities_old"),
            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
            BufferUsage::STORAGE | BufferUsage::COPY_DST,
            |_| {},
        );

        let mut types_vec = Vec::with_capacity(num_points as usize);
        for _ in 0..num_points {
            types_vec.push(thread_rng().gen_range(0..ruleset.num_point_types));
        }
        let types_vec = types_vec;
        let types = create_buffer(
            &device,
            Some("types"),
            num_points as u64 * std::mem::size_of::<PointType>() as u64,
            BufferUsage::UNIFORM,
            |types: &mut Buffer| {
                let slice = types.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                for i in 0..num_points {
                    let type_ = types_vec[i as usize];
                    cursor.write_all(&type_.to_le_bytes()).unwrap();
                }
            },
        );

        let cache_max_r = create_buffer(
            &device,
            Some("cache_max_r"),
            (ruleset.num_point_types
                * ruleset.num_point_types
                * std::mem::size_of::<Radius>() as u32) as u64,
            BufferUsage::UNIFORM,
            |cache_max_r: &mut Buffer| {
                let slice = cache_max_r.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                for y in 0..ruleset.num_point_types {
                    for x in 0..ruleset.num_point_types {
                        let max_r = ruleset.max_r[types_vec[y as usize] as usize]
                            [types_vec[x as usize] as usize];
                        cursor.write_all(&max_r.to_le_bytes()).unwrap();
                    }
                }
            },
        );

        let cache_min_r = create_buffer(
            &device,
            Some("cache_min_r"),
            (ruleset.num_point_types
                * ruleset.num_point_types
                * std::mem::size_of::<Radius>() as u32) as u64,
            BufferUsage::UNIFORM,
            |cache_min_r: &mut Buffer| {
                let slice = cache_min_r.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                for y in 0..ruleset.num_point_types {
                    for x in 0..ruleset.num_point_types {
                        let min_r = ruleset.min_r[types_vec[y as usize] as usize]
                            [types_vec[x as usize] as usize];
                        cursor.write_all(&min_r.to_le_bytes()).unwrap();
                    }
                }
            },
        );

        let cache_attraction = create_buffer(
            &device,
            Some("attraction"),
            (ruleset.num_point_types
                * ruleset.num_point_types
                * std::mem::size_of::<Attraction>() as u32) as u64,
            BufferUsage::UNIFORM,
            |cache_attraction: &mut Buffer| {
                let slice = cache_attraction.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                for y in 0..ruleset.num_point_types {
                    for x in 0..ruleset.num_point_types {
                        let attraction = ruleset.attractions[types_vec[y as usize] as usize]
                            [types_vec[x as usize] as usize];
                        cursor.write_all(&attraction.to_le_bytes()).unwrap();
                    }
                }
            },
        );

        let globals = create_buffer(
            &device,
            Some("globals"),
            (std::mem::size_of_val(&num_points)
                + std::mem::size_of_val(&ruleset.num_point_types)
                + std::mem::size_of_val(&ruleset.friction)) as u64,
            BufferUsage::UNIFORM,
            |globals| {
                let slice = globals.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                cursor.write_all(&num_points.to_le_bytes()).unwrap();
                cursor
                    .write_all(&ruleset.num_point_types.to_le_bytes())
                    .unwrap();
                cursor.write_all(&ruleset.friction.to_le_bytes()).unwrap();
            },
        );

        // Bind groups
        // 0: positions
        // 1: positions_old
        // 2: velocities
        // 3: velocities_old
        // 4: types
        // 5: cache_max_r
        // 6: cache_min_r
        // 7: cache_attraction
        // 8: globals
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("step_bindgroup_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (num_points * std::mem::size_of::<f32>() as u32 * 2) as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (num_points * std::mem::size_of::<PointType>() as u32) as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (ruleset.num_point_types
                                * ruleset.num_point_types
                                * std::mem::size_of::<Radius>() as u32)
                                as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (ruleset.num_point_types
                                * ruleset.num_point_types
                                * std::mem::size_of::<Radius>() as u32)
                                as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (ruleset.num_point_types
                                * ruleset.num_point_types
                                * std::mem::size_of::<Attraction>() as u32)
                                as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            (std::mem::size_of_val(&num_points)
                                + std::mem::size_of_val(&ruleset.num_point_types)
                                + std::mem::size_of_val(&ruleset.friction))
                                as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("step_bindgroup"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer {
                        buffer: &positions,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer {
                        buffer: &positions_old,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer {
                        buffer: &velocities,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer {
                        buffer: &velocities_old,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Buffer {
                        buffer: &types,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Buffer {
                        buffer: &cache_max_r,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Buffer {
                        buffer: &cache_min_r,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::Buffer {
                        buffer: &cache_attraction,
                        offset: 0,
                        size: None,
                    },
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::Buffer {
                        buffer: &globals,
                        offset: 0,
                        size: None,
                    },
                },
            ],
        });
        // Pipeline
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("compute_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            })),
            module: &device.create_shader_module(&ShaderModuleDescriptor {
                label: Some("compute_shader"),
                source: ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
                flags: ShaderFlags::VALIDATION,
            }),
            entry_point: "main",
        });

        Self {
            positions,
            positions_old,
            velocities,
            velocities_old,
            num_points,
            walls,
            cache_max_r,
            cache_min_r,
            globals,
            cache_attraction,
            types,
            ruleset,
            bind_group_layout,
            bind_group,
            pipeline,
            step_buffer: None,
        }
    }
    pub fn step(&mut self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("step"),
        });
        encoder.copy_buffer_to_buffer(
            &self.positions,
            0,
            &self.positions_old,
            0,
            self.num_points as u64 * std::mem::size_of::<f32>() as u64 * 2,
        );
        encoder.copy_buffer_to_buffer(
            &self.velocities,
            0,
            &self.velocities_old,
            0,
            self.num_points as u64 * std::mem::size_of::<f32>() as u64 * 2,
        );
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("step_pass"),
        });
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.set_pipeline(&self.pipeline);
        // Dispatch
        let workgroups = (self.num_points as f32 / WORKGROUP_SIZE as f32).ceil() as u32;
        compute_pass.dispatch(workgroups, 1, 1);
        drop(compute_pass);
        let cmd = encoder.finish();
        queue.submit(Some(cmd));
    }
}
