use egui::{Color32, FullOutput, ViewportId};
use egui_wgpu::ScreenDescriptor;
use egui_winit::EventResponse;
use glam::Vec2;
use glass::{
    pipelines::QuadPipeline,
    texture::Texture,
    wgpu::{BlendState, ColorTargetState, ColorWrites, Extent3d, TextureFormat, TextureUsages},
    window::GlassWindow,
    // winit::{
    //     event::Event,
    //     event_loop::{ActiveEventLoop, EventLoop},
    // },
    GlassApp,
    GlassContext,
    RenderData,
};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use wgpu::{CommandBuffer, CommandEncoder, RenderPass, StoreOp, SurfaceTexture, TextureView};
use winit::{event::MouseButton, keyboard::KeyCode};
// use winit_input_helper::WinitInputHelper;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{
    camera::Camera, circle_pipeline::CirclePipeline, color::Color, fluid_sim::FluidScene,
    post_processing::PostProcessing, rectangle_pipeline::RectanglePipeline, timer::Timer,
};

pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;

pub struct FluidSimApp {
    num_inputs: usize,
    num_renders: usize,
    fluid_sim: FluidSimState,
    gui: Option<GuiState>,
}

struct FluidSimState {
    circle_pipeline: Option<CirclePipeline>,
    rectangle_pipeline: Option<RectanglePipeline>,
    quad_pipeline: Option<QuadPipeline>,
    post_processing: Option<PostProcessing>,
    render_target: Option<Texture>,
    camera: Camera,
    fluid_scene: Arc<Mutex<FluidScene>>,
    timer: Timer,
}

impl FluidSimApp {
    pub fn new(context: &mut GlassContext) -> FluidSimApp {
        puffin::set_scopes_on(true);

        FluidSimApp {
            num_inputs: 0,
            num_renders: 0,
            fluid_sim: FluidSimState {
                circle_pipeline: Some(CirclePipeline::new(
                    context.device(),
                    ColorTargetState {
                        format: GlassWindow::default_surface_format(),

                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    },
                )),
                rectangle_pipeline: None,
                quad_pipeline: None,
                render_target: None,
                post_processing: None,
                camera: Camera::new([WIDTH as f32, HEIGHT as f32]),
                fluid_scene: Arc::new(Mutex::new(FluidScene::new(WIDTH as f32, HEIGHT as f32))),
                timer: Timer::new(),
            },
            gui: None,
        }
    }
}

impl GlassApp for FluidSimApp {
    fn start(&mut self, event_loop: &ActiveEventLoop, context: &mut GlassContext) {
        self.gui = Some(GuiState::new(event_loop, context));

        self.fluid_sim.render_target = Some(create_render_target(context));
        self.fluid_sim.rectangle_pipeline = Some(RectanglePipeline::new(
            context.device(),
            ColorTargetState {
                format: TextureFormat::Bgra8UnormSrgb,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            },
        ));
        self.fluid_sim.quad_pipeline = Some(QuadPipeline::new(
            context.device(),
            ColorTargetState {
                format: GlassWindow::default_surface_format(),
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            },
        ));
        self.fluid_sim.post_processing = Some(PostProcessing::new(context));
    }

    // fn device_input(
    //     &mut self,
    //     _context: &mut GlassContext,
    //     _event_loop: &ActiveEventLoop,
    //     _device_id: winit::event::DeviceId,
    //     _event: &winit::event::DeviceEvent,
    // ) {
    //     if let winit::event::DeviceEvent::MouseMotion { delta } = _event {
    //         self.num_device_inputs += 1;

    //         // self.fluid_sim
    //         //     .fluid_scene
    //         //     .drag(Vec2::new(delta.0 as f32, -delta.1 as f32 - 400.), false);
    //     }
    // }

    fn window_input(
        &mut self,
        context: &mut GlassContext,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: &WindowEvent,
    ) {
        puffin::profile_function!();

        if let WindowEvent::CursorMoved { position, .. } = event {
            self.num_inputs += 1;

            let screen_size = context.primary_render_window().surface_size();
            let scale_factor = context.primary_render_window().window().scale_factor() as f32;

            let pos = cursor_to_world(
                Vec2::new(position.x as f32, position.y as f32) / scale_factor,
                &[
                    screen_size[0] as f32 / scale_factor,
                    screen_size[1] as f32 / scale_factor,
                ],
                &self.fluid_sim.camera,
            );

            self.fluid_sim.fluid_scene.lock().unwrap().drag(pos, false);
        }
        update_egui_with_winit_event(self, context, &window_id, event);
    }
    // fn input(
    //     &mut self,
    //     context: &mut GlassContext,
    //     _event_loop: &EventLoopWindowTarget<()>,
    //     event: &Event<()>,
    // ) {
    //     self.num_inputs += 1;
    //     // self.fluid_sim.input.update(event);
    //     // update_egui_with_winit_event(self, context, event);
    // }

    fn update(&mut self, context: &mut GlassContext) {
        puffin::profile_function!();

        context
            .primary_render_window()
            .window()
            .set_title(&format!("FPS: {:.3}", self.fluid_sim.timer.avg_fps()));
        // let (_, scroll_diff) = self.fluid_sim.input.scroll_diff();
        // if scroll_diff > 0.0 {
        //     self.fluid_sim
        //         .camera
        //         .set_scale(self.fluid_sim.camera.scale() / 1.05);
        // } else if scroll_diff < 0.0 {
        //     self.fluid_sim
        //         .camera
        //         .set_scale(self.fluid_sim.camera.scale() * 1.05);
        // }
        // if self.fluid_sim.input.window_resized().is_some()
        //     || self.fluid_sim.input.scale_factor_changed().is_some()
        // {
        //     self.resize(context);
        // }
        // // Read inputs state
        // if self.fluid_sim.input.key_pressed(KeyCode::Space) {
        //     self.fluid_sim.fluid_scene.toggle_pause();
        // }
        // if self.fluid_sim.input.key_pressed(KeyCode::KeyR) {
        //     self.fluid_sim.fluid_scene.reset();
        // }
        // if self.fluid_sim.input.key_pressed(KeyCode::KeyG) {
        //     self.fluid_sim.fluid_scene.toggle_grid();
        // }
        // if self.fluid_sim.input.key_pressed(KeyCode::KeyP) {
        //     self.fluid_sim.fluid_scene.toggle_particles();
        // }
        // if self.fluid_sim.input.key_pressed(KeyCode::KeyF) {
        //     self.fluid_sim.fluid_scene.toggle_gravity();
        // }
        // if let Some((x, y)) = self.fluid_sim.input.cursor() {
        //     let screen_size = context.primary_render_window().surface_size();
        //     let scale_factor = 1.0; //context.primary_render_window().window().scale_factor() as f32;
        //     let pos = cursor_to_world(
        //         Vec2::new(x, y) / scale_factor,
        //         &[
        //             screen_size[0] as f32 / scale_factor,
        //             screen_size[1] as f32 / scale_factor,
        //         ],
        //         &self.fluid_sim.camera,
        //     );
        //     // if self.fluid_sim.input.mouse_pressed(MouseButton::Left) {
        //     //     self.fluid_sim.fluid_scene.drag(pos, true);
        //     // }
        //     // if self.fluid_sim.input.mouse_held(MouseButton::Left) {
        //     self.fluid_sim.fluid_scene.drag(pos, false);
        //     // }
        //     // if self.fluid_sim.input.mouse_released(MouseButton::Left) {
        //     //     self.fluid_sim.fluid_scene.end_drag();
        //     // }
        // }
        // Simulate

        let fluid_scene = self.fluid_sim.fluid_scene.clone();

        thread::spawn(move || {
            fluid_scene.lock().unwrap().simulate();
        });

        // self.fluid_sim.fluid_scene.lock().unwrap().simulate();
    }

    fn render(
        &mut self,
        context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
        puffin::profile_function!();

        self.num_renders += 1;
        return Some(render(self, context, render_data));
    }

    fn end_of_frame(&mut self, _context: &mut GlassContext) {
        puffin::profile_function!();

        self.fluid_sim.timer.update();

        puffin::GlobalProfiler::lock().new_frame();
    }
}

impl FluidSimApp {
    fn resize(&mut self, context: &GlassContext) {
        let window_size = context.primary_render_window().surface_size();
        self.fluid_sim.render_target = Some(create_render_target(context));
        self.fluid_sim
            .camera
            .update(&[window_size[0] as f32, window_size[1] as f32]);
    }
}

pub fn create_render_target(context: &GlassContext) -> Texture {
    Texture::empty(
        context.device(),
        "Render Target",
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        1,
        TextureFormat::Rgba16Float,
        &Default::default(),
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    )
}

pub fn cursor_to_world(cursor_pos: Vec2, screen_size: &[f32; 2], camera: &Camera) -> Vec2 {
    (cursor_pos - Vec2::new(screen_size[0] / 2.0, screen_size[1] / 2.0))
        * camera.scale()
        // Invert y here, because we want world positions to grow up, and right
        * Vec2::new(1.0, -1.0)
        + Vec2::new(camera.pos.x, camera.pos.y)
}

struct GuiState {
    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    repaint: bool,
}

impl GuiState {
    fn new(event_loop: &ActiveEventLoop, context: &mut GlassContext) -> GuiState {
        let ctx = egui::Context::default();
        let pixels_per_point = context.primary_render_window().window().scale_factor() as f32;
        let egui_winit = egui_winit::State::new(
            ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(pixels_per_point),
            Some(context.device().limits().max_texture_dimension_2d as usize),
        );
        let renderer = egui_wgpu::Renderer::new(
            context.device(),
            GlassWindow::default_surface_format(),
            None,
            1,
            false,
        );
        GuiState {
            egui_ctx: ctx,
            egui_winit,
            renderer,
            repaint: false,
        }
    }
}

fn render(
    app: &mut FluidSimApp,
    context: &GlassContext,
    render_data: RenderData,
) -> Vec<CommandBuffer> {
    let RenderData { encoder, frame, .. } = render_data;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    render_egui(app, context, encoder, frame, &view)
}

fn render_egui(
    app: &mut FluidSimApp,
    context: &GlassContext,
    encoder: &mut CommandEncoder,
    frame: &SurfaceTexture,
    view: &TextureView,
) -> Vec<CommandBuffer> {
    puffin::profile_function!();

    let window = context.primary_render_window();
    let Some(GuiState {
        egui_ctx,
        renderer,
        egui_winit,
        ..
    }) = &mut app.gui
    else {
        return vec![];
    };
    let raw_input = egui_winit.take_egui_input(window.window());
    let FullOutput {
        shapes,
        textures_delta,
        pixels_per_point,
        ..
    } = egui_ctx.run(raw_input, |egui_ctx| {
        // egui_adjust!(egui_ctx, app.fluid_scene);
        egui::SidePanel::left("left").show(egui_ctx, |ui| {
            ui.label("Hello World!");
            puffin_egui::show_viewport_if_enabled(egui_ctx);

            ui.label(format!("fps: {:.2}", app.fluid_sim.timer.avg_fps()));
            ui.label(format!("inputs: {}", app.num_inputs));
            ui.label(format!("renders: {}", app.num_renders));
            // ui.add(egui::Slider::new(
            //     &mut app.fluid_sim.fluid_scene.dt,
            //     0.00..=0.03,
            // ));
            // ui.add(egui::Slider::new(
            //     &mut app.fluid_sim.fluid_scene.obstacle_radius,
            //     0.00..=1.,
            // ));
        });
        // Ui content
        // ui_app.ui(egui_ctx);
    });
    // creates triangles to paint
    let clipped_primitives = egui_ctx.tessellate(shapes, pixels_per_point);

    let size = window.surface_size();
    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: size,
        pixels_per_point,
    };

    // Upload all resources for the GPU.
    let user_cmd_bufs = {
        for (id, image_delta) in &textures_delta.set {
            renderer.update_texture(context.device(), context.queue(), *id, image_delta);
        }

        // Update buffers
        renderer.update_buffers(
            context.device(),
            context.queue(),
            encoder,
            &clipped_primitives,
            &screen_descriptor,
        )
    };

    // Render
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // Here you would render your scene
        render_scene(&mut app.fluid_sim, context, &mut render_pass);
        // Render Egui
        renderer.render(&mut render_pass, &*clipped_primitives, &screen_descriptor);
    }

    for id in &textures_delta.free {
        renderer.free_texture(id);
    }

    user_cmd_bufs
}

fn render_scene<'a>(
    state: &'a mut FluidSimState,
    context: &GlassContext,
    render_pass: &mut RenderPass<'a>,
) {
    puffin::profile_function!();

    // Render on render target
    // Paste render target over swapchain image
    let FluidSimState {
        circle_pipeline,
        rectangle_pipeline,
        quad_pipeline,
        post_processing,
        render_target,
        camera,
        // input,
        fluid_scene,
        ..
    } = state;
    let fluid_scene = fluid_scene.lock().unwrap();

    let circle_pipeline = circle_pipeline.as_ref().unwrap();
    let rectangle_pipeline = rectangle_pipeline.as_ref().unwrap();
    let quad_pipeline = quad_pipeline.as_ref().unwrap();
    let post_processing = post_processing.as_ref().unwrap();
    let render_target = render_target.as_ref().unwrap();
    let window = context.primary_render_window();
    let window_size = window.surface_size();
    let scale_factor = 1.0; //input.scale_factor().unwrap_or(1.0) as f32;
    let window_size_f32 = [
        window_size[0] as f32 * scale_factor,
        window_size[1] as f32 * scale_factor,
    ];
    // We don't need to submit our commands, because they get submitted after `render`.

    // let rt_view = render_target
    //     .texture
    //     .create_view(&wgpu::TextureViewDescriptor::default());
    // Draw scene to render target
    {
        // let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //     label: None,
        //     color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //         view: &rt_view,
        //         resolve_target: None,
        //         ops: wgpu::Operations {
        //             load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        //             store: StoreOp::Store,
        //         },
        //     })],
        //     depth_stencil_attachment: None,
        //     timestamp_writes: None,
        //     occlusion_query_set: None,
        // });

        let view_proj = camera.view_proj().to_cols_array_2d();
        // Draw bounds
        rectangle_pipeline.draw(
            render_pass,
            view_proj,
            [0.0, 0.0],
            Color32::RED.into(),
            WIDTH as f32,
            HEIGHT as f32,
            2.0 / HEIGHT as f32,
            0.01,
        );

        // Draw circle(s)
        if fluid_scene.show_particles {
            let radius = fluid_scene.render_radius();
            for i in 0..fluid_scene.fluid.num_particles {
                let tank_pos = fluid_scene.fluid.particle_pos[i];
                let pos = fluid_scene.render_pos(tank_pos);
                let color = fluid_scene.fluid.particle_color[i];
                circle_pipeline.draw(
                    render_pass,
                    view_proj,
                    pos.into(),
                    Color {
                        color: [color.x, color.y, color.z, 1.0],
                    },
                    radius,
                    radius,
                    0.01,
                );
            }
        }

        if fluid_scene.show_grid {
            let size = fluid_scene.render_cell_size();
            for x in 0..fluid_scene.fluid.f_num_x {
                for y in 0..fluid_scene.fluid.f_num_y {
                    let fluid_pos = Vec2::new(
                        (x as f32 + 0.5) * fluid_scene.fluid.h,
                        (y as f32 + 0.5) * fluid_scene.fluid.h,
                    );
                    let pos = fluid_scene.render_pos(fluid_pos);
                    let i = x * fluid_scene.fluid.f_num_y + y;
                    let color = fluid_scene.fluid.cell_color[i];
                    rectangle_pipeline.draw(
                        render_pass,
                        view_proj,
                        pos.into(),
                        Color {
                            color: [color.x, color.y, color.z, 0.3],
                        },
                        size,
                        size,
                        size * 0.5,
                        0.01,
                    );
                }
            }
        }

        // Obstacle
        let pos = fluid_scene.render_pos(fluid_scene.obstacle_pos);
        let radius = fluid_scene.render_obstacle_radius();
        circle_pipeline.draw(
            render_pass,
            view_proj,
            pos.into(),
            Color {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            radius,
            radius,
            0.01,
        );
    }

    // Post Processing
    // post_processing.run(context, encoder, render_target);
    // let post_processed_target = post_processing.output();

    // let main_view = frame
    //     .texture
    //     .create_view(&wgpu::TextureViewDescriptor::default());
    // let render_target_bind_group = quad_pipeline.create_bind_group(
    //     context.device(),
    //     &post_processed_target.views[0],
    //     &post_processed_target.sampler,
    // );
    // Draw render target over swapchain image
    // {
    //     let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    //         label: None,
    //         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
    //             view: &main_view,
    //             resolve_target: None,
    //             ops: wgpu::Operations {
    //                 load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
    //                 store: StoreOp::Store,
    //             },
    //         })],
    //         depth_stencil_attachment: None,
    //         timestamp_writes: None,
    //         occlusion_query_set: None,
    //     });

    //     quad_pipeline.draw(
    //         render_pass,
    //         &render_target_bind_group,
    //         // Center
    //         [0.0, 0.0, 0.0, 0.0],
    //         camera.centered_projection().to_cols_array_2d(),
    //         window_size_f32,
    //         1.0,
    //     );
    // }
}

fn update_egui_with_winit_event(
    app: &mut FluidSimApp,
    context: &mut GlassContext,
    window_id: &WindowId,
    event: &WindowEvent,
) {
    let gui = &mut app.gui;
    if let Some(window) = context.render_window(*window_id) {
        let EventResponse { consumed, repaint } = gui
            .as_mut()
            .unwrap()
            .egui_winit
            .on_window_event(window.window(), event);
        // gui.repaint = repaint;
        // Skip input if event was consumed by egui
        if consumed {
            return;
        }
    }
}
