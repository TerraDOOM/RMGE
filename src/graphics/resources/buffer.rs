use std::mem::ManuallyDrop;
use std::rc::Rc;

use gfx_hal::{
    adapter::{Adapter, PhysicalDevice},
    buffer::{CreationError as BufferCreationError, Usage as BufferUsage},
    device::Device,
    memory::{Properties, Requirements, Segment},
    pool::CommandPool,
    Backend, MemoryTypeId,
};

use crate::error::{BufferKind, BufferOp, Error, MemoryError, MemoryKind};

#[derive(Debug)]
pub struct Memory<B: Backend, D: Device<B>> {
    pub device: Rc<ManuallyDrop<D>>,
    pub memory: ManuallyDrop<B::Memory>,
    pub size: u64,
}

impl<B: Backend, D: Device<B>> Memory<B, D> {
    pub fn new(
        device: Rc<ManuallyDrop<D>>,
        adapter: &Adapter<B>,
        properties: Properties,
        reqs: Requirements,
        memorykind: MemoryKind,
    ) -> Result<Self, Error> {
        unsafe {
            let memory_type_id = adapter
                .physical_device
                .memory_properties()
                .memory_types
                .iter()
                .enumerate()
                .find(|&(id, memory_type)| {
                    reqs.type_mask & (1 << id) != 0 && memory_type.properties.contains(properties)
                })
                .map(|(id, _)| MemoryTypeId(id))
                .ok_or(Error::MemoryError(
                    MemoryError::NoSupportedMemory,
                    memorykind,
                ))?;

            let memory =
                ManuallyDrop::new(device.allocate_memory(memory_type_id, reqs.size).map_err(
                    |e| Error::MemoryError(MemoryError::AllocationError(e), memorykind),
                )?);

            let size = reqs.size;

            Ok(Memory {
                device,
                memory,
                size,
            })
        }
    }
}

impl<B: Backend, D: Device<B>> Drop for Memory<B, D> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .free_memory(ManuallyDrop::into_inner(std::ptr::read(&self.memory)));
        }
    }
}

#[derive(Debug)]
pub struct Buffer<B: Backend, D: Device<B>> {
    pub device: Rc<ManuallyDrop<D>>,
    pub buffer: ManuallyDrop<B::Buffer>,
    _phantom: std::marker::PhantomData<D>,
}

impl<B: Backend, D: Device<B>> Buffer<B, D> {
    pub fn new(
        device: Rc<ManuallyDrop<D>>,
        size: u64,
        buffer_usage: BufferUsage,
    ) -> Result<Self, BufferCreationError> {
        let buffer = unsafe { ManuallyDrop::new(device.create_buffer(size, buffer_usage)?) };
        Ok(Self {
            device,
            buffer,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Bind this buffer to some memory. Returns the old memory it was bound to, if any
    pub fn bind_to_memory(
        &mut self,
        mem: &Memory<B, D>,
        offset: u64,
    ) -> Result<(), gfx_hal::device::BindError> {
        unsafe {
            self.device
                .bind_buffer_memory(&mem.memory, offset, &mut self.buffer)?;
        }
        Ok(())
    }
}

impl<B: Backend, D: Device<B>> std::ops::Drop for Buffer<B, D> {
    fn drop(&mut self) {
        unsafe {
            use std::ptr::read;

            self.device
                .destroy_buffer(ManuallyDrop::into_inner(read(&self.buffer)));
        }
    }
}

/*pub struct BufferView<B: Backend, D: Device<B>, T> {
device: Rc<ManuallyDrop<D>>,
ptr: *mut T,
}*/
