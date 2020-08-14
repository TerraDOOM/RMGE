use gfx_hal::{
    device::Device as DeviceTrait,
    image::Extent,
    window::{Swapchain, SwapchainConfig},
    Backend,
};

use std::rc::Rc;

#[derive(Debug)]
pub struct SwapchainData<B: Backend> {
    pub device: Rc<B::Device>,
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
        device: Rc<B::Device>,
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

    pub unsafe fn get_current_image(&mut self) -> Result<(u32, usize), &'static str> {
        let (image_index, _check_if_you_need_this) = self
            .swapchain
            .acquire_image(
                u64::max_value(),
                Some(&self.available_semaphores.as_ref().ok_or("fail")?[self.current_frame]),
                Some(&self.fences.as_ref().ok_or("")?[self.current_frame]),
            )
            .map_err(|_| "Couldn't acquire an image from the swapchain!")?;
        Ok((image_index, image_index as usize))
    }

    pub unsafe fn create_framebuffers(
        &mut self,
        device: &B::Device,
        render_pass: &B::RenderPass,
    ) -> Result<(), &'static str> {
        self.framebuffers = self
            .image_views
            .as_ref()
            .ok_or("No image views on this swapchain")?
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
                    .map_err(|_| "Failed to create a framebuffer!")
            })
            .collect::<Result<Vec<_>, &str>>()?;
        Ok(())
    }

    pub fn advance_frame(&mut self) {
        self.current_frame = match &self.available_semaphores {
            Some(semaphores) => (self.current_frame + 1) % semaphores.len(),
            None => 0,
        };
    }
}
