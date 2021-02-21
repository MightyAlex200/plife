use std::{convert::TryInto, fmt::Debug, mem::size_of, num::NonZeroU64};

use wgpu::*;

pub const VEC2_SIZE: usize = size_of::<f32>() * 2;
pub const VEC3_SIZE: usize = size_of::<f32>() * 3;

pub struct BindableBuffer {
    pub buffer: Buffer,
    pub size: u64,
    pub stages: ShaderStage,
    pub uniform: bool,
}

impl BindableBuffer {
    pub fn new<T: TryInto<u64>>(
        device: &Device,
        usage: BufferUsage,
        stages: ShaderStage,
        uniform: bool,
        size: T,
        map_function: impl FnOnce(&mut Buffer),
    ) -> Self
    where
        <T as TryInto<u64>>::Error: Debug,
    {
        let size = size.try_into().unwrap();
        let mut buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: true,
        });
        map_function(&mut buffer);
        buffer.unmap();
        Self {
            buffer,
            size,
            stages,
            uniform,
        }
    }

    pub fn bind_group_layout_entry(&self, i: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding: i,
            visibility: self.stages,
            ty: BindingType::Buffer {
                ty: if self.uniform {
                    BufferBindingType::Uniform
                } else {
                    BufferBindingType::Storage { read_only: false }
                },
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(self.size),
            },
            count: None,
        }
    }

    pub fn bind_group_entry(&self, i: u32) -> BindGroupEntry {
        BindGroupEntry {
            binding: i as u32,
            resource: BindingResource::Buffer {
                buffer: &self.buffer,
                offset: 0,
                size: NonZeroU64::new(self.size),
            },
        }
    }

    pub fn bind_group_layout(device: &Device, iter: &[&Self]) -> BindGroupLayout {
        let vec = iter
            .into_iter()
            .enumerate()
            .map(|(i, &buf)| buf.bind_group_layout_entry(i as u32))
            .collect::<Vec<_>>();
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &vec,
        })
    }

    pub fn bind_group(device: &Device, iter: &[&Self]) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &Self::bind_group_layout(&device, iter),
            entries: &iter
                .into_iter()
                .enumerate()
                .map(|(i, &buf)| buf.bind_group_entry(i as u32))
                .collect::<Vec<_>>(),
        })
    }
}
