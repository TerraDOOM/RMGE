use std::mem::ManuallyDrop;
use std::rc::Rc;

use gfx_hal::{adapter::Adapter, device::Device, pool::CommandPool, Backend};

pub mod buffer;
pub mod geometry;
pub mod textures;

#[derive(Debug)]
pub struct ResourceManager<B: Backend, D: Device<B>> {
    pub geometry_buffer: geometry::GeometryBuffer<B, D>,
}

impl<B: Backend, D: Device<B>> ResourceManager<B, D> {
    pub fn new(
        device: Rc<ManuallyDrop<D>>,
        adapter: &Adapter<B>,
        pool: &mut impl CommandPool<B>,
        queue: &mut B::CommandQueue,
    ) -> Result<Self, crate::error::Error> {
        Ok(Self {
            geometry_buffer: geometry::GeometryBuffer::new(device, adapter, pool, queue)?,
        })
    }
}
