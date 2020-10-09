use core::mem::ManuallyDrop;

use gfx_hal::{
    command::{
        ClearColor, ClearValue, CommandBuffer as CommandBufferTrait, CommandBufferFlags, Level,
        SubpassContents,
    },
    device::Device as DeviceTrait,
    format::{Aspects, Swizzle},
    image::{Layout, SubresourceRange, ViewKind},
    pass::{Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc},
    pool::CommandPool as CommandPoolTrait,
    pso::PipelineStage,
    queue::{CommandQueue, QueueGroup, Submission},
    window::Swapchain,
    Backend,
};

use super::swapchain_data::SwapchainData;
use crate::error::{Error, ErrorKind};

use arrayvec::ArrayVec;
use std::rc::Rc;

#[derive(Debug)]
pub struct DeviceData<B: Backend> {
    pub adapter_index: usize,
    pub device: Rc<ManuallyDrop<B::Device>>,
    pub queue: QueueGroup<B>,
    pub swapchains: Vec<SwapchainData<B>>,
    pub render_passes: Vec<B::RenderPass>,
}

impl<B: Backend> DeviceData<B> {
    pub fn from(adapter_index: usize, device: B::Device, queue: QueueGroup<B>) -> Self {
        Self {
            adapter_index,
            device: Rc::new(ManuallyDrop::new(device)),
            queue,
            swapchains: vec![],
            render_passes: vec![],
        }
    }
    //make this index safe
    pub fn add_semaphores(&mut self, swapchain_index: usize) -> Result<(), Error> {
        let image_count = self.swapchains[swapchain_index].config.image_count;
        let device = &self.device;
        self.swapchains[swapchain_index].fences = Some(
            (0..image_count)
                .map(|_| {
                    device.create_fence(true).map_err(|e| Error {
                        description: format!("{}", e),
                        error_kind: ErrorKind::FenceCreationError,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        self.swapchains[swapchain_index].available_semaphores = Some(
            (0..image_count)
                .map(|_| {
                    device.create_semaphore().map_err(|e| Error {
                        description: format!("{}", e),
                        error_kind: ErrorKind::SemaphoreCreationError,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        self.swapchains[swapchain_index].finished_semaphores = Some(
            (0..image_count)
                .map(|_| {
                    device.create_semaphore().map_err(|e| Error {
                        description: format!("{}", e),
                        error_kind: ErrorKind::SemaphoreCreationError,
                    })
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
                    .map_err(|e| Error {
                        description: format!("{}", e),
                        error_kind: ErrorKind::RenderPassCreationError,
                    })?
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
                        .map_err(|e| Error {
                            description: format!("{}", e),
                            error_kind: ErrorKind::ImageViewCreationError,
                        })
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
        buffers
    }

    pub fn reset_current_fence(&self, swapchain_index: usize) -> Result<(), Error> {
        unsafe {
            self.device
                .wait_for_fence(
                    &self.swapchains[swapchain_index]
                        .fences
                        .as_ref()
                        .ok_or(Error {
                            description: "Could not get fence".to_string(),
                            error_kind: ErrorKind::FenceError,
                        })?[self.swapchains[swapchain_index].current_frame],
                    u64::max_value(),
                )
                .map_err(|e| Error {
                    description: format!("Failed to wait on the fence! ({})", e),
                    error_kind: ErrorKind::FenceError,
                })?;
            self.device
                .reset_fence(
                    &self.swapchains[swapchain_index]
                        .fences
                        .as_ref()
                        .ok_or(Error {
                            description: "Could not get fence".to_string(),
                            error_kind: ErrorKind::FenceError,
                        })?[self.swapchains[swapchain_index].current_frame],
                )
                .map_err(|e| Error {
                    description: format!("Couldn't reset the fence! ({})", e),
                    error_kind: ErrorKind::FenceError,
                })?;
        }
        Ok(())
    }

    pub fn clear_frame(
        &mut self,
        color: [f32; 4],
        command_buffers: &mut [B::CommandBuffer],
    ) -> Result<(), Error> {
        // Advance the frame _before_ we start using the `?` operator
        self.swapchains[0].advance_frame();

        let (i_u32, i_usize) = unsafe { self.swapchains[0].get_current_image()? };

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
            buffer.finish();
        }

        // SUBMISSION AND PRESENT
        let command_buffers = &command_buffers[i_usize..=i_usize];
        let wait_semaphores: ArrayVec<[_; 1]> = [(
            &self.swapchains[0]
                .available_semaphores
                .as_ref()
                .ok_or(Error {
                    description: "couldn't get semaphores".to_string(),
                    error_kind: ErrorKind::SubmissionError,
                })?[self.swapchains[0].current_frame],
            PipelineStage::COLOR_ATTACHMENT_OUTPUT,
        )]
        .into();

        let signal_semaphores: ArrayVec<[_; 1]> = [&self.swapchains[0]
            .finished_semaphores
            .as_ref()
            .ok_or(Error {
                description: "couldn't get finished semaphores".to_string(),
                error_kind: ErrorKind::SubmissionError,
            })?[self.swapchains[0].current_frame]]
        .into();
        // yes, you have to write it twice like this. yes, it's silly.
        let present_wait_semaphores: ArrayVec<[_; 1]> = [&self.swapchains[0]
            .finished_semaphores
            .as_ref()
            .ok_or(Error {
                description: "couldn't get finished semaphores".to_string(),
                error_kind: ErrorKind::SubmissionError,
            })?[self.swapchains[0].current_frame]]
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
                    &self.swapchains[0].fences.as_ref().ok_or(Error {
                        description: "failed to get fences".to_string(),
                        error_kind: ErrorKind::SubmissionError,
                    })?[self.swapchains[0].current_frame],
                ),
            );
            self.swapchains[0]
                .swapchain
                .present(the_command_queue, i_u32, present_wait_semaphores)
                .map_err(|e| Error {
                    description: format!("Failed to present into the swapchain! ({})", e),
                    error_kind: ErrorKind::SubmissionError,
                })?
        };
        Ok(())
    }
}
