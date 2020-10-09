use core::mem::ManuallyDrop;

use gfx_hal::{
    device::Device as DeviceTrait,
    image::Extent,
    window::{Swapchain, SwapchainConfig},
    Backend,
};

use crate::error::{Error, ErrorKind};

use std::rc::Rc;

#[derive(Debug)]
pub struct SwapchainData<B: Backend> {
    pub device: Rc<ManuallyDrop<B::Device>>,
    pub swapchain: B::Swapchain,
    pub backbuffer: Vec<B::Image>,
    pub config: SwapchainConfig,
    pub fences: Option<Vec<B::Fence>>,
    pub available_semaphores: Option<Vec<B::Semaphore>>,
    pub finished_semaphores: Option<Vec<B::Semaphore>>,
    pub current_frame: usize,
    pub image_views: Option<Vec<B::ImageView>>,
    pub framebuffers: Vec<B::Framebuffer>,
}

impl<B: Backend> SwapchainData<B> {
    #![allow(clippy::too_many_arguments)]
    pub fn from(
        device: Rc<ManuallyDrop<B::Device>>,
        swapchain: B::Swapchain,
        backbuffer: Vec<B::Image>,
        config: SwapchainConfig,
        fences: Option<Vec<B::Fence>>,
        available_semaphores: Option<Vec<B::Semaphore>>,
        finished_semaphores: Option<Vec<B::Semaphore>>,
        image_views: Option<Vec<B::ImageView>>,
        framebuffers: Vec<B::Framebuffer>,
    ) -> Self {
        Self {
            device,
            swapchain,
            backbuffer,
            config,
            fences,
            available_semaphores,
            finished_semaphores,
            current_frame: 0,
            image_views,
            framebuffers,
        }
    }

    pub unsafe fn get_current_image(&mut self) -> Result<(u32, usize), Error> {
        let (image_index, _check_if_you_need_this) = self
            .swapchain
            .acquire_image(
                u64::max_value(),
                Some(
                    &self.available_semaphores.as_ref().ok_or(Error {
                        description: "failed to get semaphores".to_string(),
                        error_kind: ErrorKind::SemaphoreError,
                    })?[self.current_frame],
                ),
                Some(
                    &self.fences.as_ref().ok_or(Error {
                        description: "failed to get fences".to_string(),
                        error_kind: ErrorKind::FenceError,
                    })?[self.current_frame],
                ),
            )
            .map_err(|e| Error {
                description: format!("Couldn't acquire an image from the swapchain! ({})", e),
                error_kind: ErrorKind::SwapchainError,
            })?;
        Ok((image_index, image_index as usize))
    }

    pub unsafe fn create_framebuffers(
        &mut self,
        device: &B::Device,
        render_pass: &B::RenderPass,
    ) -> Result<(), Error> {
        self.framebuffers = self
            .image_views
            .as_ref()
            .ok_or(Error {
                description: "No image views on this swapchain".to_string(),
                error_kind: ErrorKind::SwapchainError,
            })?
            .iter()
            .map(|image_view| {
                device
                    .create_framebuffer(
                        render_pass,
                        vec![image_view],
                        Extent {
                            width: self.config.extent.width as u32,
                            height: self.config.extent.height as u32,
                            depth: 1,
                        },
                    )
                    .map_err(|e| Error {
                        description: format!("Failed to create a framebuffer! ({})", e),
                        error_kind: ErrorKind::FramebufferCreationError,
                    })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(())
    }

    pub fn advance_frame(&mut self) {
        self.current_frame = match &self.available_semaphores {
            Some(semaphores) => (self.current_frame + 1) % semaphores.len(),
            None => 0,
        };
    }
}
