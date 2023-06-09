use crate::{
    engine::VoxelEngine,
    scene::Scene,
    window::{Window, WindowSettings},
};
use std::time::Instant;

pub fn init(settings: WindowSettings) {
    std::env::set_var("RUST_LOG", "info, wgpu_core=error");
    env_logger::init();

    let (mut window, renderer, event_loop) = Window::new(settings);
    window.grab_cursor(true);
    let size = window.size();

    let mut engine = VoxelEngine {
        renderer,
        window,
        locked_input: false,
    };
    let mut scene = Scene::new(&engine.renderer, size.0 as f32, size.1 as f32);
    let mut last_render_time = Instant::now();

    event_loop.run(move |event, _, flow| {
        engine.renderer_mut().gui.platform.handle_event(&event);
        if !engine.locked_input {
            scene.handle_input_events(&event);
            engine.renderer_mut().input(&event);
        }

        match event {
            winit::event::Event::MainEventsCleared => {
                let scale_factor = engine.window.scale_factor();
                let dt = last_re nder_time.elapsed();
                engine.renderer_mut().update(&scene);
                scene.update(dt);
                last_render_time = Instant::now();
                match engine.renderer_mut().render(scale_factor, dt.as_secs_f32()) {
                    Ok(_) => (),
                    Err(e) => log::error!("Rendering Error: {:?}", e),
                }
            }
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::KeyboardInput { input, .. } => {
                    if input.state == winit::event::ElementState::Pressed {
                        engine.on_key_pressed(input.virtual_keycode);
                    }
                }
                winit::event::WindowEvent::CloseRequested => {
                    *flow = winit::event_loop::ControlFlow::Exit
                }
                winit::event::WindowEvent::Resized(size) => {
                    engine.renderer_mut().resize(&mut scene, size);
                }
                winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    engine.renderer_mut().resize(&mut scene, *new_inner_size);
                }
                _ => (),
            },
            _ => (),
        }
    });
}
