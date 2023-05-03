use ui::EguiInstance;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod renderer;
mod ui;
fn main() {
    run();
}

pub fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("Failed to create window");

    let gui = EguiInstance::new(&window);
    let mut renderer = pollster::block_on(renderer::Renderer::new(&window, gui));

    event_loop.run(move |generic_event, _, control_flow| {
        renderer.gui.platform.handle_event(&generic_event);

        match generic_event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !renderer.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => {
                            renderer.resize(*size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            renderer.resize(**new_inner_size);
                        }
                        _ => {}
                    };
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                renderer.update();
                match renderer.render(&window) {
                    Ok(_) => {}
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
