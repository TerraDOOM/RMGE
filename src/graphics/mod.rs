#![warn(
    clippy::cast_lossless,
    clippy::checked_conversions,
    clippy::copy_iterator,
    clippy::default_trait_access,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_into_iter_loop,
    clippy::explicit_iter_loop,
    clippy::filter_map,
    clippy::filter_map_next,
    clippy::find_map,
    clippy::if_not_else,
    clippy::inline_always,
    clippy::items_after_statements,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::map_flatten,
    clippy::match_same_arms,
    clippy::maybe_infinite_iter,
    clippy::mut_mut,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::non_ascii_literal,
    clippy::map_unwrap_or,
    clippy::pub_enum_variant_names,
    clippy::redundant_closure_for_method_calls,
    clippy::same_functions_in_if_condition,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::type_repetition_in_bounds,
    clippy::unicode_not_nfc,
    clippy::unseparated_literal_suffix,
    clippy::unused_self,
    clippy::used_underscore_binding
)]

mod device_data;
mod pipeline_data;
mod resources;
mod swapchain_data;

use crate::error::Error;
use device_data::DeviceData;
use pipeline_data::PipelineData;
use resources::ResourceManager;
use swapchain_data::SwapchainData;

use std::mem::{self, ManuallyDrop};
use std::rc::Rc;

use log::{error, info};

use gfx_hal::{
    adapter::{Adapter, Gpu, PhysicalDevice},
    device::Device as DeviceTrait,
    format::{ChannelType, Format},
    pool::{CommandPool, CommandPoolCreateFlags},
    queue::QueueFamily as QueueFamilyTrait,
    window::{Surface, SwapchainConfig},
    Backend, Features, Instance,
};

use raw_window_handle::HasRawWindowHandle;

#[derive(Debug)]
struct CommandData<B: Backend> {
    device: Rc<ManuallyDrop<B::Device>>,
    command_pool: B::CommandPool,
    command_buffers: Vec<B::CommandBuffer>,
}

impl<B: Backend> CommandData<B> {
    unsafe fn new(device_data: &mut DeviceData<B>) -> Result<Self, Error> {
        let mut command_pool = device_data
            .device
            .create_command_pool(
                device_data.queue.family,
                CommandPoolCreateFlags::RESET_INDIVIDUAL,
            )
            .map_err(|_| Error::CommandPoolCreationError)?;
        let command_buffers = device_data.create_command_buffers(&mut command_pool);

        Ok(CommandData {
            device: device_data.device.clone(),
            command_pool,
            command_buffers,
        })
    }
}

#[derive(Debug)]
pub struct Context<B: Backend> {
    resources: Vec<ResourceManager<B, B::Device>>,
    instance: ManuallyDrop<B::Instance>,
    surface: ManuallyDrop<B::Surface>,
    adapters: Vec<Adapter<B>>,
    devices: Vec<DeviceData<B>>,
    command_pools: Vec<CommandData<B>>,
}

impl<B: Backend> Context<B> {
    pub fn build<W: HasRawWindowHandle>(window: &W, name: &str) -> Result<Self, Error> {
        let mut context = Self::from_window(window, name)?;
        context.add_device()?;
        context.add_swapchain(0)?;
        context.add_semaphores(0, 0)?;
        context.devices[0].add_render_pass()?;
        context.devices[0].add_image_views(0)?;
        context.devices[0].add_framebuffers(0, 0)?;
        context.add_command_pool(0)?;
        context.devices[0].add_graphics_pipeline(0, 0)?;
        context.add_resource_manager(0, 0)?;

        Ok(context)
    }

    pub fn from_window<W: HasRawWindowHandle>(window: &W, name: &str) -> Result<Self, Error> {
        let raw_instance =
            B::Instance::create(name, 1).map_err(|e| Error::InstanceCreationError(e))?;

        let surface = unsafe {
            raw_instance
                .create_surface(window)
                .map_err(|e| Error::SurfaceCreationError(e))?
        };

        let adapters = raw_instance
            .enumerate_adapters()
            .into_iter()
            .map(|mut a| {
                a.queue_families = a
                    .queue_families
                    .into_iter()
                    .filter(|qf| {
                        qf.queue_type().supports_graphics() && surface.supports_queue_family(qf)
                    })
                    .collect();
                a
            })
            .filter(|a| !a.queue_families.is_empty())
            .collect::<Vec<_>>();

        Ok(Self {
            instance: ManuallyDrop::new(raw_instance),
            surface: ManuallyDrop::new(surface),
            adapters,
            resources: vec![],
            devices: vec![],
            command_pools: vec![],
        })
    }

    fn add_device(&mut self) -> Result<(), Error> {
        use crate::error::QueueGroupError;

        let (
            index,
            Gpu {
                device,
                queue_groups,
            },
            _family,
        ) = self
            .adapters
            .iter()
            .enumerate()
            .find_map(|(index, a)| {
                a.queue_families.iter().find_map(|qf| unsafe {
                    a.physical_device
                        .open(&[(&qf, &[1.0; 1])], Features::empty())
                        .ok()
                        .map(|gpu| (index, gpu, qf))
                })
            })
            .ok_or(Error::QueueGroupError(QueueGroupError::QueueGroupNotFound))?;

        info!("Chosen adapter name: {}", self.adapters[index].info.name);

        // TODO: Make this good
        let queue_group = queue_groups
            .into_iter()
            .next()
            .ok_or(Error::QueueGroupError(QueueGroupError::OwnershipFailed))?;

        if queue_group.queues.is_empty() {
            return Err(Error::QueueGroupError(QueueGroupError::NoCommandQueues));
        };

        self.devices
            .push(DeviceData::from(index, device, queue_group));

        Ok(())
    }

    fn add_swapchain(&mut self, device_index: usize) -> Result<(), Error> {
        let DeviceData {
            adapter_index,
            device,
            ..
        } = self
            .devices
            .get(device_index)
            .ok_or(Error::MissingDevice(device_index))?;

        let surface_capabilities = self
            .surface
            .capabilities(&self.adapters[*adapter_index].physical_device);

        let &present_mode = {
            use gfx_hal::window::PresentMode;
            let present_modes = surface_capabilities.present_modes;

            [
                PresentMode::MAILBOX,
                PresentMode::FIFO,
                PresentMode::RELAXED,
                PresentMode::IMMEDIATE,
            ]
            .iter()
            .find(|pm| present_modes.contains(**pm))
            .ok_or(Error::SwapchainError(
                crate::error::SwapchainError::NoPresentMode,
            ))?
        };

        info!("Chosen present mode: {:?}", present_mode);

        let preferred_formats = self
            .surface
            .supported_formats(&self.adapters[*adapter_index].physical_device);

        let format = match preferred_formats {
            None => Format::Rgba8Srgb,
            Some(formats) => match formats
                .iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .cloned()
            {
                Some(srgb_format) => srgb_format,
                None => formats.get(0).cloned().ok_or(Error::SwapchainError(
                    crate::error::SwapchainError::NoPresentMode,
                ))?,
            },
        };

        let swapchain_config = SwapchainConfig::from_caps(
            &surface_capabilities,
            format,
            *surface_capabilities.extents.end(),
        )
        .with_present_mode(present_mode);

        let (swapchain, backbuffer) = unsafe {
            device
                .create_swapchain(&mut self.surface, swapchain_config.clone(), None)
                .map_err(|e| {
                    Error::SwapchainError(crate::error::SwapchainError::CreationError(e))
                })?
        };
        let device = self.devices[0].device.clone();

        self.devices[0].swapchains.push(SwapchainData::from(
            device,
            swapchain,
            backbuffer,
            swapchain_config,
            None,
            None,
            None,
            None,
            vec![],
        ));
        Ok(())
    }

    fn add_resource_manager(
        &mut self,
        device_index: usize,
        command_pool_index: usize,
    ) -> Result<(), Error> {
        let adapter = &self.adapters[self
            .devices
            .get_mut(device_index)
            .ok_or(Error::MissingDevice(device_index))?
            .adapter_index];
        let command_pool = self
            .command_pools
            .get_mut(command_pool_index)
            .ok_or(Error::MissingCommandPool(command_pool_index))?;

        let queue = &mut self
            .devices
            .get_mut(device_index)
            .ok_or(Error::MissingDevice(device_index))?
            .queue
            .queues[0];

        self.resources.push(ResourceManager::new(
            command_pool.device.clone(),
            adapter,
            &mut command_pool.command_pool,
            queue,
        )?);

        Ok(())
    }

    fn add_semaphores(&mut self, device_index: usize, swapchain_index: usize) -> Result<(), Error> {
        self.devices
            .get_mut(device_index)
            .ok_or(Error::MissingDevice(device_index))?
            .add_semaphores(swapchain_index)
    }

    fn add_command_pool(&mut self, device_index: usize) -> Result<(), Error> {
        unsafe {
            self.command_pools.push(CommandData::new(
                self.devices
                    .get_mut(device_index)
                    .ok_or(Error::MissingDevice(device_index))?,
            )?);
        }
        Ok(())
    }

    pub fn clear(&mut self, color: [f32; 4]) -> Result<(), Error> {
        self.devices
            .get_mut(0)
            .ok_or(Error::MissingDevice(0))?
            .clear_frame(
                color,
                &mut self
                    .command_pools
                    .get_mut(0)
                    .ok_or(Error::MissingCommandPool(0))?
                    .command_buffers,
            )
    }

    pub fn draw(scene: &mut crate::scene::SceneTree) {}

    pub fn draw_quad(
        &mut self,
        quad: crate::geometry::Quad,
        clear_color: [f32; 4],
    ) -> Result<(), Error> {
        use crate::geometry::{Quad, Vec3};

        self.resources
            .get_mut(0)
            .ok_or(Error::MissingResourceManager(0))?
            .geometry_buffer
            .add_quad(0, 0, quad)?;

        self.devices
            .get_mut(0)
            .ok_or(Error::MissingDevice(0))?
            .draw(
                clear_color,
                &self.resources[0], // at this point we know it exists or we would've returned already
                &mut self
                    .command_pools
                    .get_mut(0)
                    .ok_or(Error::MissingCommandPool(0))?
                    .command_buffers,
            )?;

        Ok(())
    }
}

impl<B: Backend> std::ops::Drop for Context<B> {
    fn drop(&mut self) {
        // we drop the result since an error here would be quite unrecoverable
        // we can't really return an error message

        for device_data in &self.devices {
            let _ = device_data.device.wait_idle();
        }

        info!(target: "rmge", "waited for all devices");

        for mut command_data in self.command_pools.drain(..) {
            unsafe {
                command_data.command_pool.reset(true);
                command_data
                    .device
                    .destroy_command_pool(command_data.command_pool);
            }
        }

        for mut resource in self.resources.drain(..) {}

        info!(target: "rmge", "destroyed all command queues");

        for DeviceData {
            mut pipelines,
            device,
            swapchains,
            render_passes,
            queue: _,
            adapter_index: _,
        } in self.devices.drain(..)
        {
            for data in pipelines.drain(..) {
                mem::drop(data);
            }

            for render_pass in render_passes {
                unsafe { device.destroy_render_pass(render_pass) };
            }
            info!(target: "rmge", "render passes destroyed");

            for swapchain_data in swapchains {
                let SwapchainData {
                    swapchain,
                    backbuffer,
                    fences,
                    available_semaphores,
                    finished_semaphores,
                    image_views,
                    framebuffers,
                    device: _, // we already have the correct device (hopefully)
                    config: _,
                    current_frame: _,
                } = swapchain_data;
                unsafe {
                    for fence in fences.unwrap_or_else(Vec::new) {
                        device.destroy_fence(fence);
                    }

                    info!(target: "rmge", "fences destroyed");

                    for semaphore in available_semaphores.unwrap_or_else(Vec::new) {
                        device.destroy_semaphore(semaphore);
                    }

                    info!(target: "rmge", "available semaphores destroyed");

                    for semaphore in finished_semaphores.unwrap_or_else(Vec::new) {
                        device.destroy_semaphore(semaphore);
                    }

                    info!(target: "rmge", "finished semaphores destroyed");

                    for image_view in image_views.unwrap_or_else(Vec::new) {
                        device.destroy_image_view(image_view);
                    }

                    info!(target: "rmge", "image views destroyed");

                    //for image in backbuffer {
                    //    device.destroy_image(image);
                    //}

                    info!(target: "rmge", "backbuffer images destroyed");

                    for framebuffer in framebuffers {
                        device.destroy_framebuffer(framebuffer);
                    }

                    info!(target: "rmge", "framebuffers destroyed");

                    device.destroy_swapchain(swapchain);

                    info!(target: "rmge", "swapchain destroyed")
                }
            }

            info!(target: "rmge", "finished dropping swapchains");

            match Rc::try_unwrap(device) {
                Ok(mut dev) => {
                    unsafe { ManuallyDrop::drop(&mut dev) };
                    info!(target: "rmge", "dropped device");
                }
                Err(rc) => {
                    // if this happens then everything is probably failing or I forgot to drop something
                    error!(target: "rmge", "There were still {} alive `Rc`s to device!", Rc::strong_count(&rc));
                }
            }
        }
        unsafe {
            self.instance
                .destroy_surface(ManuallyDrop::into_inner(std::ptr::read(&self.surface)));

            ManuallyDrop::drop(&mut self.instance);
        }

        info!(target: "rmge", "dropped instance");
    }
}
