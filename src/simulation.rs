use std::{
    io::{Cursor, Write},
    mem::size_of,
};

use rand::{thread_rng, Rng};
use wgpu::*;

use crate::{serialize::*, util::*};

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

pub enum Walls {
    None,
    Square(f32),
    Wrapping(f32),
}

pub struct Simulation {
    pub num_points: u32,
    pub ruleset: Ruleset,
    pub walls: Walls,
    pub positions: BindableBuffer,
    pub globals: BindableBuffer,
    pub types: BindableBuffer,
    positions_old: BindableBuffer,
    bind_group: BindGroup,
    pipeline: ComputePipeline,
}

impl Simulation {
    pub fn from_config(device: &Device, config: Config) -> Self {
        let (ruleset, walls, points) = config.sample();
        let num_points = points.len() as u32;
        // Buffers
        // TODO: BindableBuffer::using_cursor
        let positions = BindableBuffer::new(
            &device,
            BufferUsage::STORAGE | BufferUsage::COPY_SRC | BufferUsage::VERTEX,
            ShaderStage::all(),
            false,
            num_points as usize * VEC2_SIZE,
            |positions: &mut Buffer| {
                let slice = positions.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                for point in points {
                    cursor.write_all(&point.0.to_le_bytes()).unwrap();
                    cursor.write_all(&point.1.to_le_bytes()).unwrap();
                }
            },
        );

        let positions_old = BindableBuffer::new(
            &device,
            BufferUsage::STORAGE | BufferUsage::COPY_DST,
            ShaderStage::all(),
            false,
            num_points as usize * VEC2_SIZE,
            |_| {},
        );

        let velocities = BindableBuffer::new(
            &device,
            BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            ShaderStage::COMPUTE,
            false,
            num_points as usize * VEC2_SIZE,
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

        let mut types_vec = Vec::with_capacity(num_points as usize);
        for _ in 0..num_points {
            types_vec.push(thread_rng().gen_range(0..ruleset.num_point_types));
        }

        let types_vec = types_vec;
        let types = BindableBuffer::new(
            &device,
            BufferUsage::STORAGE,
            ShaderStage::all(),
            false,
            num_points as usize * size_of::<PointType>(),
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

        let num_type_pairs = ruleset.num_point_types * ruleset.num_point_types;

        let cache_max_r = BindableBuffer::new(
            &device,
            BufferUsage::STORAGE,
            ShaderStage::COMPUTE,
            false,
            num_type_pairs as usize * size_of::<Radius>(),
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

        let cache_min_r = BindableBuffer::new(
            &device,
            BufferUsage::STORAGE,
            ShaderStage::COMPUTE,
            false,
            num_type_pairs as usize * size_of::<Radius>(),
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

        let cache_attraction = BindableBuffer::new(
            &device,
            BufferUsage::STORAGE,
            ShaderStage::COMPUTE,
            false,
            num_type_pairs as usize * size_of::<Attraction>(),
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

        let globals = BindableBuffer::new(
            &device,
            BufferUsage::UNIFORM,
            ShaderStage::all(),
            true,
            size_of::<u32>()
                + size_of::<PointType>()
                + size_of::<Friction>()
                + size_of::<u32>()
                + size_of::<f32>(),
            |globals| {
                let slice = globals.slice(..);
                let mut view = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *view);
                cursor.write_all(&num_points.to_le_bytes()).unwrap();
                cursor
                    .write_all(&ruleset.num_point_types.to_le_bytes())
                    .unwrap();
                cursor.write_all(&ruleset.friction.to_le_bytes()).unwrap();
                let (wrapping, dist) = match walls {
                    Walls::None => (false, 0.0),
                    Walls::Square(dist) => (false, dist),
                    Walls::Wrapping(dist) => (true, dist),
                };
                cursor
                    .write_all(&if wrapping {
                        1u32.to_le_bytes()
                    } else {
                        0u32.to_le_bytes()
                    })
                    .unwrap();
                cursor.write_all(&dist.to_le_bytes()).unwrap();
            },
        );

        let buffers = [
            &positions,
            &positions_old,
            &velocities,
            &types,
            &cache_max_r,
            &cache_min_r,
            &cache_attraction,
            &globals,
        ];

        // Bind groups
        // 0: positions
        // 1: positions_old
        // 2: velocities
        // 4: types
        // 5: cache_max_r
        // 6: cache_min_r
        // 7: cache_attraction
        // 8: globals
        let bind_group_layout = BindableBuffer::bind_group_layout(&device, &buffers);
        let bind_group = BindableBuffer::bind_group(&device, &buffers);
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
            num_points,
            walls,
            globals,
            types,
            ruleset,
            bind_group,
            pipeline,
        }
    }
    pub fn step(&mut self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("step"),
        });
        encoder.copy_buffer_to_buffer(
            &self.positions.buffer,
            0,
            &self.positions_old.buffer,
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
        device.poll(Maintain::Wait);
    }
}
