#![feature(backtrace)]

extern crate rmge;
extern crate winit;
#[macro_use]
extern crate log;

use rmge::geometry::{Quad, Vec3};
use rmge::Renderer;

use log::LevelFilter;
use simple_logger::SimpleLogger;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    SimpleLogger::new()
        .with_module_level("gfx_backend_vulkan", LevelFilter::Warn)
        .init()
        .unwrap();

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

    let mut context = match Renderer::new(&window, "something") {
        Ok(context) => context,
        Err(e) => {
            use std::error::Error;

            let back = e.backtrace();
            eprintln!(
                "Error creating context\nerror: {}\nsource: {:?}\nbacktrace: {:?}",
                e,
                e.source(),
                back,
            );
            std::process::exit(1);
        }
    };

    event_loop.run(move |e, _, control_flow| match e {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::MainEventsCleared => {
            context
                .draw_quad(
                    Quad {
                        points: [
                            Vec3::new(0.0, 0.0, 0.0),
                            Vec3::new(0.5, 0.0, 0.0),
                            Vec3::new(0.5, 0.5, 0.0),
                            Vec3::new(0.0, 0.5, 0.0),
                        ],
                    },
                    [0.0, 1.0, 0.0, 1.0],
                )
                .expect("failed to clear screen");
        }
        _ => {}
    });
}
