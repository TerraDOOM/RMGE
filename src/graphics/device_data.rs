use core::mem::ManuallyDrop;

use log::{info, warn};

use gfx_hal::{
    buffer::{IndexBufferView, SubRange},
    command::{
        ClearColor, ClearValue, CommandBuffer as CommandBufferTrait, CommandBufferFlags, Level,
        SubpassContents,
    },
    device::Device as DeviceTrait,
    format::{Aspects, Format, Swizzle},
    image::{Layout, SubresourceRange, ViewKind},
    pass::{Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc},
    pool::CommandPool as CommandPoolTrait,
    pso::{
        AttributeDesc, DescriptorPool, Element, PipelineStage, VertexBufferDesc, VertexInputRate,
    },
    queue::{CommandQueue, QueueGroup, Submission},
    window::Swapchain,
    Backend, IndexType,
};

use super::pipeline_data::PipelineData;
use super::resources::ResourceManager;
use super::swapchain_data::SwapchainData;
use crate::error::*;

use arrayvec::ArrayVec;
use std::{mem, rc::Rc};

#[derive(Debug)]
pub struct DeviceData<B: Backend> {
    pub adapter_index: usize,
    pub device: Rc<ManuallyDrop<B::Device>>,
    pub queue: QueueGroup<B>,
    pub swapchains: Vec<SwapchainData<B>>,
    pub render_passes: Vec<B::RenderPass>,
    pub pipelines: Vec<PipelineData<B, B::Device>>,
}

impl<B: Backend> DeviceData<B> {
    pub fn from(adapter_index: usize, device: B::Device, queue: QueueGroup<B>) -> Self {
        Self {
            adapter_index,
            device: Rc::new(ManuallyDrop::new(device)),
            queue,
            swapchains: vec![],
            render_passes: vec![],
            pipelines: vec![],
        }
    }
    //make this index safe
    pub fn add_semaphores(&mut self, swapchain_index: usize) -> Result<(), Error> {
        let image_count = self.swapchains[swapchain_index].config.image_count;
        let device = &self.device;
        self.swapchains[swapchain_index].fences = Some(
            (0..image_count)
                .map(|n| {
                    let mut fence = device
                        .create_fence(true)
                        .map_err(|e| Error::FenceCreationError)?;
                    unsafe {
                        device.set_fence_name(&mut fence, &format!("fence #{}", n));
                    }
                    Ok(fence)
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        self.swapchains[swapchain_index].available_semaphores = Some(
            (0..image_count)
                .map(|_| {
                    device
                        .create_semaphore()
                        .map_err(|e| Error::SemaphoreCreationError)
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        self.swapchains[swapchain_index].finished_semaphores = Some(
            (0..image_count)
                .map(|_| {
                    device
                        .create_semaphore()
                        .map_err(|e| Error::SemaphoreCreationError)
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        Ok(())
    }

    pub fn add_render_pass(&mut self) -> Result<(), Error> {
        self.render_passes.push({
            let color_attachment = Attachment {
                format: Some(self.swapchains[0].config.format),
                samples: 1,
                ops: AttachmentOps {
                    load: AttachmentLoadOp::Clear,
                    store: AttachmentStoreOp::Store,
                },
                stencil_ops: AttachmentOps::DONT_CARE,
                layouts: Layout::Undefined..Layout::Present,
            };
            let subpass = SubpassDesc {
                colors: &[(0, Layout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: &[],
                resolves: &[],
                preserves: &[],
            };
            unsafe {
                self.device
                    .create_render_pass(&[color_attachment], &[subpass], &[])
                    .map_err(|e| Error::RenderPassCreationError)?
            }
        });
        Ok(())
    }

    pub fn add_image_views(&mut self, swapchain_index: usize) -> Result<(), Error> {
        self.swapchains[swapchain_index].image_views = Some(
            self.swapchains[swapchain_index]
                .backbuffer
                .iter()
                .map(|image| unsafe {
                    self.device
                        .create_image_view(
                            &image,
                            ViewKind::D2,
                            self.swapchains[swapchain_index].config.format,
                            Swizzle::NO,
                            SubresourceRange {
                                aspects: Aspects::COLOR,
                                levels: 0..1,
                                layers: 0..1,
                            },
                        )
                        .map_err(|e| Error::ImageViewCreationError)
                })
                .collect::<Result<Vec<_>, Error>>()?,
        );
        Ok(())
    }

    pub fn add_framebuffers(
        &mut self,
        swapchain_index: usize,
        render_pass_index: usize,
    ) -> Result<(), Error> {
        unsafe {
            self.swapchains[swapchain_index]
                .create_framebuffers(&self.device, &self.render_passes[render_pass_index])?
        };
        Ok(())
    }

    pub fn create_command_buffers(
        &mut self,
        command_pool: &mut B::CommandPool,
    ) -> Vec<B::CommandBuffer> {
        let num_buffers = self.swapchains[0].framebuffers.len();
        let mut buffers = Vec::new();
        unsafe {
            command_pool.allocate(num_buffers, Level::Primary, &mut buffers);
        }

        for (c, buf) in buffers.iter_mut().enumerate() {
            unsafe {
                self.device
                    .set_command_buffer_name(buf, &format!("drawing buffer #{}", c));
            }
        }

        buffers
    }

    pub fn add_graphics_pipeline(
        &mut self,
        swapchain_index: usize,
        render_pass_index: usize,
    ) -> Result<(), Error> {
        let vertex_buffers = vec![
            // the vertices
            VertexBufferDesc {
                binding: 0,
                stride: mem::size_of::<f32>() as u32 * 3,
                rate: VertexInputRate::Vertex,
            },
            // the texture indices
            VertexBufferDesc {
                binding: 1,
                stride: mem::size_of::<u32>() as u32,
                rate: VertexInputRate::Instance(1),
            },
        ];

        let attributes = vec![
            AttributeDesc {
                location: 0,
                binding: 0,
                element: Element {
                    format: Format::Rgb32Sfloat,
                    offset: 0,
                },
            },
            AttributeDesc {
                location: 1,
                binding: 1,
                element: Element {
                    format: Format::R32Uint,
                    offset: 0,
                },
            },
        ];

        let data = PipelineData::new(
            self.device.clone(),
            self.swapchains[swapchain_index].config.extent.to_extent(),
            &self.render_passes[render_pass_index],
            vertex_buffers,
            attributes,
        )?;

        Ok(self.pipelines.push(data))
    }

    pub fn reset_current_fence(&self, swapchain_index: usize) -> Result<(), Error> {
        unsafe {
            self.device
                .wait_for_fence(
                    &self.swapchains[swapchain_index]
                        .fences
                        .as_ref()
                        .ok_or(Error::FenceError(FenceOp::Acquire))?
                        [self.swapchains[swapchain_index].current_frame],
                    u64::max_value(),
                )
                .map_err(|e| Error::FenceError(FenceOp::Wait))?;
            self.device
                .reset_fence(
                    &self.swapchains[swapchain_index]
                        .fences
                        .as_ref()
                        .ok_or(Error::FenceError(FenceOp::Acquire))?
                        [self.swapchains[swapchain_index].current_frame],
                )
                .map_err(|e| Error::FenceError(FenceOp::Reset))?;
        }
        Ok(())
    }

    pub fn draw(
        &mut self,
        color: [f32; 4],
        resources: &ResourceManager<B, B::Device>,
        command_buffers: &mut [B::CommandBuffer],
    ) -> Result<(), Error> {
        self.swapchains[0].advance_frame();

        // as said in clear_frame, we do this twice to reset the fence after get_current_image signals it
        self.reset_current_fence(0)?;

        let (i_u32, i_usize) = unsafe { self.swapchains[0].get_current_image()? };

        self.reset_current_fence(0)?;

        unsafe {
            let clear_values = [ClearValue {
                color: ClearColor { float32: color },
            }];
            let index_buffer_view = IndexBufferView {
                buffer: &*resources.geometry_buffer.quad_index_buffer.buffer,
                range: SubRange::WHOLE,
                index_type: IndexType::U16,
            };

            let pipeline = &self
                .pipelines
                .get(0)
                .ok_or(Error::MissingPipeline(0))?
                .graphics_pipeline;
            let buffer = &mut command_buffers[i_usize];

            buffer.reset(true);
            buffer.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);
            buffer.begin_render_pass(
                &self.render_passes[0],
                &self.swapchains[0].framebuffers[i_usize],
                self.swapchains[0].config.extent.to_extent().rect(),
                clear_values.iter(),
                SubpassContents::Inline,
            );
            buffer.bind_graphics_pipeline(pipeline);
            buffer.bind_index_buffer(index_buffer_view);
            buffer.bind_vertex_buffers(
                0,
                vec![
                    (
                        &*resources.geometry_buffer.quad_buffer.buffer,
                        SubRange::WHOLE,
                    ),
                    (
                        &*resources.geometry_buffer.quad_instance_buffer.buffer,
                        SubRange::WHOLE,
                    ),
                ],
            );
            buffer.draw_indexed(0..6, 0, 0..4);
            buffer.end_render_pass();
            buffer.finish();
        }

        // SUBMISSION AND PRESENT
        let command_buffers = &command_buffers[i_usize..=i_usize];
        let wait_semaphores: ArrayVec<[_; 1]> = [(
            &self.swapchains[0]
                .available_semaphores
                .as_ref()
                .ok_or(Error::SubmissionError)?[self.swapchains[0].current_frame],
            PipelineStage::COLOR_ATTACHMENT_OUTPUT,
        )]
        .into();

        let signal_semaphores: ArrayVec<[_; 1]> = [&self.swapchains[0]
            .finished_semaphores
            .as_ref()
            .ok_or(Error::SubmissionError)?[self.swapchains[0].current_frame]]
        .into();
        // yes, you have to write it twice like this. yes, it's silly.
        let present_wait_semaphores: ArrayVec<[_; 1]> = [&self.swapchains[0]
            .finished_semaphores
            .as_ref()
            .ok_or(Error::SubmissionError)?[self.swapchains[0].current_frame]]
        .into();

        let submission = Submission {
            command_buffers,
            wait_semaphores,
            signal_semaphores,
        };

        let the_command_queue = &mut self.queue.queues[0];

        unsafe {
            the_command_queue.submit(
                submission,
                Some(
                    &self.swapchains[0]
                        .fences
                        .as_ref()
                        .ok_or(Error::FenceError(FenceOp::Acquire))?
                        [self.swapchains[0].current_frame],
                ),
            );
            self.swapchains[0]
                .swapchain
                .present(the_command_queue, i_u32, present_wait_semaphores)
                .map_err(|e| Error::SubmissionError)?
        };

        Ok(())
    }

    pub fn clear_frame(
        &mut self,
        color: [f32; 4],
        command_buffers: &mut [B::CommandBuffer],
    ) -> Result<(), Error> {
        // Advance the frame _before_ we start using the `?` operator
        self.swapchains[0].advance_frame();

        // we first reset the fence for get_current_image
        self.reset_current_fence(0)?;

        let (i_u32, i_usize) = unsafe { self.swapchains[0].get_current_image()? };

        // we then reset the fence again because it was signalled by get_current_image and it needs to be unsignalled
        self.reset_current_fence(0)?;

        // RECORD COMMANDS
        unsafe {
            let buffer = &mut command_buffers[i_usize];

            let clear_values = [ClearValue {
                color: ClearColor { float32: color },
            }];

            buffer.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);
            buffer.begin_render_pass(
                &self.render_passes[0],
                &self.swapchains[0].framebuffers[i_usize],
                self.swapchains[0].config.extent.to_extent().rect(),
                clear_values.iter(),
                SubpassContents::Inline,
            );
            buffer.end_render_pass();
            buffer.finish();
        }
        {
            // SUBMISSION AND PRESENT
            let command_buffers = &command_buffers[i_usize..=i_usize];
            let wait_semaphores: ArrayVec<[_; 1]> = [(
                &self.swapchains[0]
                    .available_semaphores
                    .as_ref()
                    .ok_or(Error::SubmissionError)?[self.swapchains[0].current_frame],
                PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            )]
            .into();

            let signal_semaphores: ArrayVec<[_; 1]> = [&self.swapchains[0]
                .finished_semaphores
                .as_ref()
                .ok_or(Error::SubmissionError)?[self.swapchains[0].current_frame]]
            .into();
            // yes, you have to write it twice like this. yes, it's silly.
            let present_wait_semaphores: ArrayVec<[_; 1]> = [&self.swapchains[0]
                .finished_semaphores
                .as_ref()
                .ok_or(Error::SubmissionError)?[self.swapchains[0].current_frame]]
            .into();

            let submission = Submission {
                command_buffers,
                wait_semaphores,
                signal_semaphores,
            };

            let the_command_queue = &mut self.queue.queues[0];

            unsafe {
                the_command_queue.submit(
                    submission,
                    Some(
                        &self.swapchains[0]
                            .fences
                            .as_ref()
                            .ok_or(Error::FenceError(FenceOp::Acquire))?
                            [self.swapchains[0].current_frame],
                    ),
                );
                self.swapchains[0]
                    .swapchain
                    .present(the_command_queue, i_u32, present_wait_semaphores)
                    .map_err(|e| Error::SubmissionError)?
            };
        }

        Ok(())
    }
}
