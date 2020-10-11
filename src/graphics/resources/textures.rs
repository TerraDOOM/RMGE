use core::mem::ManuallyDrop;
use gfx_hal::{device::Device, Backend};
use std::rc::Rc;

pub struct SpriteBatch<B: Backend, D: Device<B>> {
    pub device: Rc<ManuallyDrop<D>>,
    pub memory: B::Memory,
}
