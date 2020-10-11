use core::mem::{self, ManuallyDrop};

use std::rc::Rc;

use crate::geometry::{Mat4, Quad as Quad3d, Quad2d};

use gfx_hal::{
    adapter::{Adapter, PhysicalDevice},
    buffer::Usage as BufferUsage,
    device::Device,
    memory::{Properties, Segment},
    pool::CommandPool,
    Backend, MemoryTypeId,
};

use crate::error::*;

static QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

// chosen arbitrarily, subject to change.
const DEFAULT_NUM_MATRICES: u64 = 32;
const DEFAULT_NUM_QUADS: u64 = 1024;

pub struct GeometryBuffer<B: Backend, D: Device<B>> {
    device: Rc<ManuallyDrop<D>>,
    max_matrixes: usize,
    max_quads: usize,
    allocated_mem: usize,
    pub local_memory: ManuallyDrop<B::Memory>,
    pub index_memory: ManuallyDrop<B::Memory>,
    pub matrix_buffer: ManuallyDrop<B::Buffer>,
    pub quad_buffer: ManuallyDrop<B::Buffer>,
    pub quad_index_buffer: ManuallyDrop<B::Buffer>,
}

impl<B: Backend, D: Device<B>> GeometryBuffer<B, D> {
    pub fn new<C: CommandPool<B>>(
        device: Rc<ManuallyDrop<D>>,
        adapter: &Adapter<B>,
        command_pool: &mut C,
    ) -> Result<Self, Error> {
        Self::with_size(
            device,
            adapter,
            command_pool,
            DEFAULT_NUM_MATRICES,
            DEFAULT_NUM_QUADS,
        )
    }

    pub fn with_size<C: CommandPool<B>>(
        device: Rc<ManuallyDrop<D>>,
        adapter: &Adapter<B>,
        command_pool: &mut C,
        num_matrices: u64,
        num_quads: u64,
    ) -> Result<Self, Error> {
        unsafe {
            let mut matrix_buffer = device
                .create_buffer(
                    num_matrices * mem::size_of::<Mat4>() as u64,
                    BufferUsage::VERTEX,
                )
                .map_err(|e| Error::BufferError(BufferOp::Create, BufferKind::Matrix))?;
            let mut quad_buffer: B::Buffer = match device.create_buffer(
                num_quads * mem::size_of::<Quad3d>() as u64,
                BufferUsage::VERTEX,
            ) {
                Ok(buf) => buf,
                Err(e) => {
                    device.destroy_buffer(matrix_buffer);
                    return Err(Error::BufferError(BufferOp::Create, BufferKind::Quad));
                }
            };

            let matrix_requirements = device.get_buffer_requirements(&matrix_buffer);
            let quad_requirements = device.get_buffer_requirements(&quad_buffer);

            // if it supports the matrix buffer, then we hope it also supports the quad one.
            let memory_type_id = match adapter
                .physical_device
                .memory_properties()
                .memory_types
                .iter()
                .enumerate()
                .find(|&(id, memory_type)| {
                    matrix_requirements.type_mask & (1 << id) != 0
                        && memory_type.properties.contains(Properties::CPU_VISIBLE)
                })
                .map(|(id, _)| MemoryTypeId(id))
            {
                Some(mem_type_id) => mem_type_id,
                None => {
                    device.destroy_buffer(matrix_buffer);
                    device.destroy_buffer(quad_buffer);
                    return Err(Error::MemoryError(
                        MemoryError::NoSupportedMemory,
                        MemoryKind::Geometry,
                    ));
                }
            };

            let memory = match device.allocate_memory(
                memory_type_id,
                matrix_requirements.size + quad_requirements.size,
            ) {
                Ok(memory) => memory,
                Err(e) => {
                    device.destroy_buffer(matrix_buffer);
                    device.destroy_buffer(quad_buffer);
                    return Err(Error::MemoryError(
                        MemoryError::AllocationError,
                        MemoryKind::Geometry,
                    ));
                }
            };

            if let Err(e) = device.bind_buffer_memory(&memory, 0, &mut matrix_buffer) {
                device.destroy_buffer(matrix_buffer);
                device.destroy_buffer(quad_buffer);
                device.free_memory(memory);
                return Err(Error::BufferError(BufferOp::Bind, BufferKind::Matrix));
            }

            if let Err(e) = device.bind_buffer_memory(
                &memory,
                num_matrices * mem::size_of::<Mat4>() as u64,
                &mut quad_buffer,
            ) {
                device.destroy_buffer(matrix_buffer);
                device.destroy_buffer(quad_buffer);
                device.free_memory(memory);
                return Err(Error::BufferError(BufferOp::Bind, BufferKind::Quad));
            }

            unimplemented!()
        }
    }

    // separate function to ease the error handling a little bit
    fn create_index_memory_and_buffer(
        device: &D,
        adapter: &Adapter<B>,
        command_pool: &B::CommandPool,
    ) -> Result<(B::Memory, B::Buffer), Error> {
        unsafe {
            let staging_buffer = device
                .create_buffer(
                    mem::size_of_val(&QUAD_INDICES) as u64,
                    BufferUsage::TRANSFER_DST,
                )
                .map_err(|e| Error::BufferError(BufferOp::Create, BufferKind::Staging))?;

            let staging_reqs = device.get_buffer_requirements(&staging_buffer);

            let memory_type_id = match adapter
                .physical_device
                .memory_properties()
                .memory_types
                .iter()
                .enumerate()
                .find(|&(id, memory_type)| {
                    staging_reqs.type_mask & (1 << id) != 0
                        && memory_type.properties.contains(Properties::CPU_VISIBLE)
                })
                .map(|(id, _)| MemoryTypeId(id))
            {
                Some(mem_type_id) => mem_type_id,
                None => {
                    device.destroy_buffer(staging_buffer);
                    return Err(Error::MemoryError(
                        MemoryError::NoSupportedMemory,
                        MemoryKind::Staging,
                    ));
                }
            };

            let staging_mem = match device.allocate_memory(memory_type_id, staging_reqs.size) {
                Ok(mem) => mem,
                Err(e) => {
                    device.destroy_buffer(staging_buffer);
                    return Err(Error::MemoryError(
                        MemoryError::AllocationError,
                        MemoryKind::Staging,
                    ));
                }
            };

            {
                let staging_ptr = match device.map_memory(&staging_mem, Segment::ALL) {
                    Ok(ptr) => ptr as *mut u16,
                    Err(e) => {
                        device.destroy_buffer(staging_buffer);
                        device.free_memory(staging_mem);
                        return Err(Error::MemoryError(
                            MemoryError::MappingError,
                            MemoryKind::Staging,
                        ));
                    }
                };
            }
        }

        unimplemented!()
    }
}
