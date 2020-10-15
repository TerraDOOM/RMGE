use core::mem::{self, ManuallyDrop};

use std::rc::Rc;

use gfx_hal::{
    adapter::{Adapter, PhysicalDevice},
    buffer::Usage as BufferUsage,
    command::{CommandBuffer, CommandBufferFlags, Level},
    device::Device,
    memory::{Properties, Segment},
    pool::CommandPool,
    queue::CommandQueue,
    Backend, MemoryTypeId,
};

use crate::error::*;
use crate::geometry::{Mat4, Quad as Quad3d};

use super::buffer::{Buffer, Memory};

static QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

// chosen arbitrarily, subject to change.
const DEFAULT_NUM_MATRICES: u64 = 32;
const DEFAULT_NUM_QUADS: u64 = 1024;

#[derive(Debug)]
pub struct GeometryBuffer<B: Backend, D: Device<B>> {
    device: Rc<ManuallyDrop<D>>,
    max_matrices: u64,
    max_quads: u64,
    allocated_mem: u64,
    pub geometry_memory: Memory<B, D>,
    pub index_memory: Memory<B, D>,
    pub matrix_buffer: Buffer<B, D>,
    pub quad_instance_buffer: Buffer<B, D>,
    pub quad_buffer: Buffer<B, D>,
    pub quad_index_buffer: Buffer<B, D>,
}

impl<B: Backend, D: Device<B>> GeometryBuffer<B, D> {
    pub fn new<C: CommandPool<B>>(
        device: Rc<ManuallyDrop<D>>,
        adapter: &Adapter<B>,
        command_pool: &mut C,
        command_queue: &mut B::CommandQueue,
    ) -> Result<Self, Error> {
        Self::with_size(
            device,
            adapter,
            command_pool,
            command_queue,
            DEFAULT_NUM_MATRICES,
            DEFAULT_NUM_QUADS,
        )
    }

    pub fn with_size<C: CommandPool<B>>(
        device: Rc<ManuallyDrop<D>>,
        adapter: &Adapter<B>,
        command_pool: &mut C,
        command_queue: &mut B::CommandQueue,
        num_matrices: u64,
        num_quads: u64,
    ) -> Result<Self, Error> {
        unsafe {
            let mut matrix_buffer = Buffer::new(
                device.clone(),
                num_matrices * mem::size_of::<Mat4>() as u64,
                BufferUsage::VERTEX,
            )
            .map_err(|e| Error::BufferError(BufferOp::Create(e), BufferKind::Matrix))?;

            let mut quad_instance_buffer = Buffer::new(
                device.clone(),
                num_quads * mem::size_of::<u32>() as u64,
                BufferUsage::VERTEX,
            )
            .map_err(|e| Error::BufferError(BufferOp::Create(e), BufferKind::Instance))?;

            let mut quad_buffer = Buffer::new(
                device.clone(),
                num_quads * mem::size_of::<Quad3d>() as u64,
                BufferUsage::VERTEX,
            )
            .map_err(|e| Error::BufferError(BufferOp::Create(e), BufferKind::Quad))?;

            let mut requirements = device.get_buffer_requirements(&matrix_buffer.buffer);
            let quad_instance_requirements =
                device.get_buffer_requirements(&quad_instance_buffer.buffer);
            let quad_requirements = device.get_buffer_requirements(&quad_buffer.buffer);

            requirements.size += quad_instance_requirements.size + quad_requirements.size;

            let geometry_memory = Memory::new(
                device.clone(),
                adapter,
                // CPU_VISIBLE | DEVICE_LOCAL might not always be available, but we hope it is
                Properties::CPU_VISIBLE | Properties::DEVICE_LOCAL,
                requirements,
                MemoryKind::Geometry,
            )?;

            matrix_buffer
                .bind_to_memory(&geometry_memory, 0)
                .map_err(|e| Error::BufferError(BufferOp::Bind(e), BufferKind::Matrix))?;

            quad_instance_buffer
                .bind_to_memory(
                    &geometry_memory,
                    num_matrices * mem::size_of::<Mat4>() as u64,
                )
                .map_err(|e| Error::BufferError(BufferOp::Bind(e), BufferKind::Quad))?;

            quad_buffer
                .bind_to_memory(
                    &geometry_memory,
                    num_quads * mem::size_of::<u32>() as u64
                        + num_matrices * mem::size_of::<Mat4>() as u64,
                )
                .map_err(|e| Error::BufferError(BufferOp::Bind(e), BufferKind::Quad))?;

            let (index_memory, quad_index_buffer) = Self::create_index_memory_and_buffer(
                device.clone(),
                adapter,
                command_pool,
                command_queue,
            )?;

            Ok(Self {
                device,
                max_matrices: num_matrices,
                max_quads: num_quads,
                allocated_mem: mem::size_of::<Mat4>() as u64 * num_matrices
                    + (mem::size_of::<u32>() + mem::size_of::<Quad3d>()) as u64 * num_quads,
                geometry_memory,
                index_memory,
                matrix_buffer,
                quad_instance_buffer,
                quad_buffer,
                quad_index_buffer,
            })
        }
    }

    // separate function to ease the error handling a little bit
    fn create_index_memory_and_buffer<C: CommandPool<B>>(
        device: Rc<ManuallyDrop<D>>,
        adapter: &Adapter<B>,
        command_pool: &mut C,
        command_queue: &mut B::CommandQueue,
    ) -> Result<(Memory<B, D>, Buffer<B, D>), Error> {
        unsafe {
            let mut index_buffer = Buffer::new(
                device.clone(),
                mem::size_of::<[u16; 6]>() as u64,
                BufferUsage::INDEX | BufferUsage::TRANSFER_DST,
            )
            .map_err(|e| Error::BufferError(BufferOp::Create(e), BufferKind::Index))?;

            let reqs = device.get_buffer_requirements(&index_buffer.buffer);

            let memory = Memory::new(
                device.clone(),
                adapter,
                Properties::DEVICE_LOCAL,
                reqs,
                MemoryKind::Index,
            )?;

            index_buffer
                .bind_to_memory(&memory, 0)
                .map_err(|e| Error::BufferError(BufferOp::Bind(e), BufferKind::Index))?;

            {
                let mut buffer = command_pool.allocate_one(Level::Primary);
                let fence = device.create_fence(false).expect("fence stuff");
                device.set_command_buffer_name(&mut buffer, "quad index update buffer");
                buffer.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);
                buffer.update_buffer(
                    &index_buffer.buffer,
                    0,
                    &mem::transmute::<[u16; 6], [u8; 12]>(QUAD_INDICES),
                );
                buffer.finish();
                command_queue.submit_without_semaphores(Some(&buffer), Some(&fence));
                device
                    .wait_for_fence(&fence, u64::MAX)
                    .expect("more fence stuff");
                device.destroy_fence(fence);

                buffer.reset(true);
                command_pool.free(Some(buffer));
            }

            Ok((memory, index_buffer))
        }
    }

    pub fn add_matrix(&mut self, trans: Mat4, index: usize) -> Result<(), Error> {
        unimplemented!()
    }

    pub fn add_quad(
        &mut self,
        index: usize,
        texture_index: u32,
        quad: Quad3d,
    ) -> Result<(), Error> {
        unsafe {
            let mapped_segment = Segment {
                offset: (mem::size_of::<Mat4>() as u64 * self.max_matrices
                    + mem::size_of::<u32>() as u64 * self.max_quads),
                size: None,
            };

            let quad_ptr = self
                .device
                .map_memory(&self.geometry_memory.memory, mapped_segment.clone())
                .expect("this is bad");

            use std::ptr;

            ptr::write((quad_ptr as *mut Quad3d).offset(index as isize), quad);

            self.device
                .flush_mapped_memory_ranges(Some((&*self.geometry_memory.memory, mapped_segment)))
                .expect("failed flush");
            self.device.unmap_memory(&self.geometry_memory.memory);

            Ok(())
        }
    }
}
