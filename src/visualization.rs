use crate::{
    simulation::Simulation,
    util::{BindableBuffer, VEC2_SIZE, VEC3_SIZE},
};
use async_executor::LocalExecutor;
use std::{
    io::{Cursor, Write},
    mem::size_of,
    num::NonZeroU64,
    time::{Duration, Instant},
};
use wgpu::util::*;
use wgpu::*;
use winit::{
    event::{VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct Visualization {
    pub simulation: Simulation,
    pub ticks: u64,
    pub ticks_per_frame: u16,
    vertex_buffer: BindableBuffer,
    index_buffer: BindableBuffer,
    ticks_just_now: u16,
    last_update_duration: Duration,
    pipeline: RenderPipeline,
    swapchain: SwapChain,
    sc_desc: SwapChainDescriptor,
    bind_group: BindGroup,
    render_globals: BindableBuffer,
    staging_belt: StagingBelt,
    executor: LocalExecutor<'static>,
    // Camera
    x: f32,
    y: f32,
    zoom: f32,
    last_mouse_position: Option<winit::dpi::PhysicalPosition<f64>>,
}

impl Visualization {
    const CIRCLE_VERTS: u32 = 16;
    const CIRCLE_RADIUS: f32 = 5.0;

    pub fn with_random_colors(
        device: &Device,
        adapter: &Adapter,
        surface: &Surface,
        simulation: Simulation,
    ) -> Self {
        let colors = BindableBuffer::new(
            &device,
            BufferUsage::UNIFORM,
            ShaderStage::VERTEX,
            true,
            simulation.ruleset.num_point_types as usize * VEC3_SIZE,
            |colors| {
                let slice = colors.slice(..);
                let mut range = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *range);
                for _ in 0..simulation.ruleset.num_point_types * 3 {
                    cursor
                        .write_all(&rand::random::<f32>().to_le_bytes())
                        .unwrap();
                }
            },
        );

        let render_globals = BindableBuffer::new(
            &device,
            BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            ShaderStage::VERTEX,
            true,
            size_of::<f32>() * 3 + size_of::<u32>() * 2, // x + y + + width + height + zoom
            |_| {},
        );

        let vertex_buffer = BindableBuffer::new(
            &device,
            BufferUsage::VERTEX,
            ShaderStage::VERTEX,
            false,
            VEC2_SIZE as u32 * (Self::CIRCLE_VERTS + 1),
            |vert_buf| {
                let slice = vert_buf.slice(..);
                let mut range = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *range);
                for _ in 0..2 {
                    cursor.write_all(&0.0f32.to_le_bytes()).unwrap();
                }
                for i in 0..Self::CIRCLE_VERTS {
                    let i = i as f32 / Self::CIRCLE_VERTS as f32 * 2.0 * std::f32::consts::PI;
                    let x = i.cos() * Self::CIRCLE_RADIUS;
                    let y = i.sin() * Self::CIRCLE_RADIUS;
                    cursor.write_all(&x.to_le_bytes()).unwrap();
                    cursor.write_all(&y.to_le_bytes()).unwrap();
                }
            },
        );

        let index_buffer = BindableBuffer::new(
            &device,
            BufferUsage::INDEX,
            ShaderStage::VERTEX,
            false,
            size_of::<u32>() as u32 * Self::CIRCLE_VERTS * 3,
            |idx_buf| {
                let slice = idx_buf.slice(..);
                let mut range = slice.get_mapped_range_mut();
                let mut cursor = Cursor::new(&mut *range);
                for i in 0..Self::CIRCLE_VERTS {
                    let i = i + 1;
                    cursor.write_all(&0u32.to_le_bytes()).unwrap();

                    cursor.write_all(&i.to_le_bytes()).unwrap();

                    cursor
                        .write_all(&((i % Self::CIRCLE_VERTS) + 1).to_le_bytes())
                        .unwrap();
                }
            },
        );

        let staging_belt = StagingBelt::new(render_globals.size);

        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("render_shader"),
            source: ShaderSource::Wgsl(include_str!("render.wgsl").into()),
            flags: ShaderFlags::VALIDATION,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                simulation.globals.bind_group_layout_entry(0),
                render_globals.bind_group_layout_entry(1),
                simulation.types.bind_group_layout_entry(2),
                colors.bind_group_layout_entry(3),
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("render_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                simulation.globals.bind_group_entry(0),
                render_globals.bind_group_entry(1),
                simulation.types.bind_group_entry(2),
                colors.bind_group_entry(3),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let swapchain_format = adapter.get_swap_chain_preferred_format(&surface);

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: VEC2_SIZE as u64,
                        step_mode: InputStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            format: VertexFormat::Float2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    VertexBufferLayout {
                        array_stride: VEC2_SIZE as u64,
                        step_mode: InputStepMode::Instance,
                        attributes: &[VertexAttribute {
                            format: VertexFormat::Float2,
                            offset: 0,
                            shader_location: 1,
                        }],
                    },
                ],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[swapchain_format.into()],
            }),
        });

        let sc_desc = SwapChainDescriptor {
            usage: TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: 800,
            height: 600,
            present_mode: PresentMode::Mailbox,
        };

        let swapchain = device.create_swap_chain(&surface, &sc_desc);

        Visualization {
            simulation,
            swapchain,
            sc_desc,
            bind_group,
            ticks: 0,
            ticks_per_frame: 1,
            ticks_just_now: 0,
            last_update_duration: Duration::from_millis(1),
            pipeline,
            render_globals,
            staging_belt,
            executor: LocalExecutor::new(),
            x: 0.0,
            y: 0.0,
            zoom: 0.0007,
            last_mouse_position: None,
            vertex_buffer,
            index_buffer,
        }
    }

    fn update(&mut self, device: &Device, queue: &Queue) {
        self.ticks_just_now = 0;
        let start = Instant::now();
        for _ in 0..self.ticks_per_frame {
            self.simulation.step(device, queue);
            self.ticks += 1;
            self.ticks_just_now += 1;
        }
        let end = Instant::now();
        self.last_update_duration = end - start;
    }

    fn render(&mut self, device: &Device, queue: &Queue) {
        let frame = self.swapchain.get_current_frame().unwrap().output;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("render"),
        });
        // Write render globals
        {
            let mut view = self.staging_belt.write_buffer(
                &mut encoder,
                &self.render_globals.buffer,
                0,
                NonZeroU64::new(self.render_globals.size).unwrap(),
                &device,
            );
            let mut cursor = Cursor::new(&mut *view);
            cursor.write_all(&self.x.to_le_bytes()).unwrap();
            cursor.write_all(&self.y.to_le_bytes()).unwrap();
            cursor.write_all(&self.sc_desc.width.to_le_bytes()).unwrap();
            cursor
                .write_all(&self.sc_desc.height.to_le_bytes())
                .unwrap();
            cursor.write_all(&self.zoom.to_le_bytes()).unwrap();
            drop(cursor);
            drop(view);
            self.staging_belt.finish();
        }
        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            render_pass.set_vertex_buffer(0, self.vertex_buffer.buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.simulation.positions.buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.buffer.slice(..), IndexFormat::Uint32);
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw_indexed(
                0..(Self::CIRCLE_VERTS * 3),
                0,
                0..self.simulation.num_points,
            );
        }
        queue.submit(Some(encoder.finish()));

        self.executor.spawn(self.staging_belt.recall()).detach();
    }

    fn handle_window_event(
        &mut self,
        window_event: WindowEvent,
        control_flow: &mut ControlFlow,
        device: &Device,
        surface: &Surface,
        mouse_down: &mut bool,
    ) {
        match window_event {
            WindowEvent::Resized(size) => {
                self.sc_desc.width = size.width;
                self.sc_desc.height = size.height;
                self.swapchain = device.create_swap_chain(&surface, &self.sc_desc);
            }
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                Some(VirtualKeyCode::Escape) => {
                    *control_flow = ControlFlow::Exit;
                }
                Some(VirtualKeyCode::LBracket) => {
                    if let Some(new_tps) = self.ticks_per_frame.checked_sub(1) {
                        self.ticks_per_frame = new_tps;
                    }
                }
                Some(VirtualKeyCode::RBracket) => {
                    if let Some(new_tps) = self.ticks_per_frame.checked_add(1) {
                        self.ticks_per_frame = new_tps;
                    }
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(last_pos) = self.last_mouse_position {
                    if *mouse_down {
                        let delta = winit::dpi::PhysicalPosition {
                            x: (position.x - last_pos.x)
                                / self.zoom as f64
                                / self.sc_desc.width as f64,
                            y: (position.y - last_pos.y)
                                / self.zoom as f64
                                / self.sc_desc.height as f64,
                        };
                        let smallest_dimension = self.sc_desc.width.min(self.sc_desc.height);
                        self.x -=
                            delta.x as f32 * self.sc_desc.width as f32 / smallest_dimension as f32;
                        self.y +=
                            delta.y as f32 * self.sc_desc.height as f32 / smallest_dimension as f32;
                    }
                }
                self.last_mouse_position = Some(position);
            }
            WindowEvent::MouseWheel {
                delta: winit::event::MouseScrollDelta::LineDelta(_, lines),
                phase: winit::event::TouchPhase::Moved,
                ..
            } => {
                if lines > 0.0 {
                    self.zoom *= 1.1;
                } else {
                    self.zoom /= 1.1;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let winit::event::MouseButton::Left = button {
                    *mouse_down = state == winit::event::ElementState::Pressed;
                }
            }
            _ => {}
        }
    }

    pub fn run(
        mut self,
        device: Device,
        queue: Queue,
        _window: Window,
        surface: Surface,
        event_loop: EventLoop<()>,
    ) -> ! {
        let mut mouse_down = false;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                winit::event::Event::WindowEvent {
                    event: window_event,
                    ..
                } => {
                    self.handle_window_event(
                        window_event,
                        control_flow,
                        &device,
                        &surface,
                        &mut mouse_down,
                    );
                }
                winit::event::Event::MainEventsCleared => {
                    while self.executor.try_tick() {}
                    self.update(&device, &queue);
                    self.render(&device, &queue);
                }
                winit::event::Event::LoopDestroyed => {}
                _ => {}
            }
        })
    }
}
