#![allow(dead_code)]

#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
#[cfg(feature = "default")]
use gfx_backend_vulkan as back;

pub mod error;
pub mod geometry;
pub mod graphics;
pub mod scene;
pub mod tracker;

use graphics::Context;
use raw_window_handle::HasRawWindowHandle;
use scene::{SceneNode, SceneTree};

pub struct Renderer {
    context: Context<back::Backend>,
    scenetree: SceneTree,
}

impl Renderer {
    pub fn new<W: HasRawWindowHandle>(window: &W, name: &str) -> Result<Renderer, error::Error> {
        Ok(Renderer {
            context: Context::build(window, name)?,
            scenetree: SceneTree::new(SceneNode::new(geometry::Mat4::identity())),
        })
    }

    pub fn draw_quad(&mut self, quad: geometry::Quad, color: [f32; 4]) -> Result<(), error::Error> {
        self.context.draw_quad(quad, color)
    }

    pub fn clear(&mut self, color: [f32; 4]) -> Result<(), error::Error> {
        self.context.clear(color)
    }

    pub fn submit(&mut self) {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
