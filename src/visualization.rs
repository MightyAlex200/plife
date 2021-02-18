use crate::{
    simulation::Simulation,
    util::{BindableBuffer, VEC3_SIZE},
};
use std::{
    io::{Cursor, Write},
    time::{Duration, Instant},
};
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
    ticks_just_now: u16,
    last_update_duration: Duration,
    pipeline: RenderPipeline,
    swapchain: SwapChain,
    sc_desc: SwapChainDescriptor,
    bind_group: BindGroup,
}

impl Visualization {
    pub fn with_random_colors(
        device: &Device,
        adapter: &Adapter,
        surface: &Surface,
        simulation: Simulation,
    ) -> Self {
        let colors = BindableBuffer::new(
            &device,
            BufferUsage::UNIFORM,
            ShaderStage::FRAGMENT,
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

        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("render_shader"),
            source: ShaderSource::Wgsl(include_str!("render.wgsl").into()),
            flags: ShaderFlags::VALIDATION,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                simulation.positions.bind_group_layout_entry(0),
                simulation.globals.bind_group_layout_entry(1),
                simulation.types.bind_group_layout_entry(2),
                colors.bind_group_layout_entry(3),
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("render_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                simulation.positions.bind_group_entry(0),
                simulation.globals.bind_group_entry(1),
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
                buffers: &[],
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

    fn render(&self, device: &Device, queue: &Queue) {
        let frame = self.swapchain.get_current_frame().unwrap().output;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("render"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }

    fn handle_window_event(
        &mut self,
        window_event: WindowEvent,
        control_flow: &mut ControlFlow,
        device: &Device,
        surface: &Surface,
    ) {
        match window_event {
            WindowEvent::Resized(size) => {
                self.sc_desc.width = size.width;
                self.sc_desc.height = size.height;
                self.swapchain = device.create_swap_chain(&surface, &self.sc_desc);
            }
            WindowEvent::Moved(_) => {}
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                    *control_flow = ControlFlow::Exit;
                }
            }
            WindowEvent::CursorMoved {
                position: _position,
                ..
            } => {}
            WindowEvent::MouseWheel {
                delta: _delta,
                phase: _phase,
                ..
            } => {}
            WindowEvent::MouseInput {
                state: _state,
                button: _button,
                ..
            } => {}
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
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                winit::event::Event::WindowEvent {
                    event: window_event,
                    ..
                } => {
                    self.handle_window_event(window_event, control_flow, &device, &surface);
                }
                winit::event::Event::MainEventsCleared => {
                    self.update(&device, &queue);
                    self.render(&device, &queue);
                }
                winit::event::Event::LoopDestroyed => {}
                _ => {}
            }
        })
    }
}

// impl EventHandler for Visualization {
//     fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) {
//         if y > 0.0 {
//             self.zoom *= 1.1;
//         } else if y < 0.0 {
//             self.zoom /= 1.1;
//         }
//     }

//     fn mouse_motion_event(&mut self, ctx: &mut Context, _x: f32, _y: f32, dx: f32, dy: f32) {
//         if input::mouse::button_pressed(ctx, MouseButton::Left) {
//             self.camera_offset += Vec2::from((dx, dy)) / self.zoom;
//         }
//     }

//     fn key_down_event(
//         &mut self,
//         _ctx: &mut Context,
//         keycode: KeyCode,
//         _keymods: ggez::event::KeyMods,
//         repeat: bool,
//     ) {
//         if repeat {
//             return;
//         }
//         let res = match keycode {
//             KeyCode::LBracket => self.ticks_per_frame.checked_sub(1),
//             KeyCode::RBracket => self.ticks_per_frame.checked_add(1),
//             _ => return,
//         };
//         if let Some(new) = res {
//             self.ticks_per_frame = new;
//         }
//     }

//     fn draw(&mut self, ctx: &mut Context) -> GameResult {
//         graphics::clear(ctx, graphics::Color::BLACK);

//         let mut mesh_builder = MeshBuilder::new();

//         let mut points = vec![Complex::new(0.0f32, 0.0f32); self.simulation.num_points as usize];
//         self.simulation.positions.host(&mut points);

//         let mut point_types = vec![PointType::default(); self.simulation.num_points as usize];
//         self.simulation.types.host(&mut point_types);

//         for (point, point_type) in points.into_iter().zip(point_types) {
//             mesh_builder.circle(
//                 DrawMode::fill(),
//                 [point.re, point.im],
//                 3.0,
//                 0.1,
//                 self.colors[point_type as usize].clone(),
//             )?;
//         }

//         let mesh = mesh_builder.build(ctx)?;

//         let draw_params = DrawParam {
//             trans: Transform::Values {
//                 dest: (self.camera_offset * self.zoom
//                     + Vec2::from(ggez::graphics::size(ctx)) / 2.0)
//                     .into(),
//                 rotation: 0.0,
//                 scale: Vec2::splat(self.zoom).into(),
//                 offset: Vec2::new(0.0, 0.0).into(),
//             },
//             ..Default::default()
//         };

//         graphics::draw(ctx, &mesh, draw_params)?;

//         let fps = (1.0 / ggez::timer::delta(ctx).as_secs_f32()) as u16;
//         let tps = ((1.0 / self.last_update_duration.as_secs_f32()) as u16).min(fps)
//             * self.ticks_per_frame;
//         let target_tps = self.ticks_per_frame * 60;
//         let fps_text = Text::new(format!(
//             "fps: {}\ntps: {}\ntarget tps: {}",
//             fps, tps, target_tps
//         ));

//         graphics::draw::<_, graphics::DrawParam>(ctx, &fps_text, Default::default())?;

//         graphics::present(ctx)
//     }
// }
