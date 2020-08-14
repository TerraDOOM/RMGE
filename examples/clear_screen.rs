extern crate rmge;
extern crate winit;
use rmge::Renderer;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::new();

    let wb = winit::window::WindowBuilder::new()
        .with_min_inner_size(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
            64.0, 64.0,
        )))
        .with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(
            640, 480,
        )))
        .with_title("quad".to_string());
    let window = wb.build(&event_loop).expect("failed to build window");

    let mut context = Renderer::build(&window, "something").expect("failed to build context");
    event_loop.run(move |e, _, control_flow| match e {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        _ => context
            .clear([0.0, 1.0, 0.0, 1.0])
            .expect("failed to build context"),
    });
}
