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
mod swapchain_data;

use device_data::DeviceData;
use swapchain_data::SwapchainData;

use core::mem::ManuallyDrop;

use gfx_hal::{
    adapter::{Adapter, Gpu, PhysicalDevice},
    device::Device as DeviceTrait,
    format::{ChannelType, Format},
    pool::CommandPoolCreateFlags,
    queue::QueueFamily as QueueFamilyTrait,
    window::{Surface, SwapchainConfig},
    Backend, Features, Instance,
};

use raw_window_handle::HasRawWindowHandle;

#[derive(Debug)]
pub struct Context<B: Backend> {
    instance: ManuallyDrop<B::Instance>,
    surface: B::Surface,
    adapters: Vec<Adapter<B>>,
    devices: Vec<DeviceData<B>>,
    command_pools: Vec<B::CommandPool>,
    command_buffers: Vec<B::CommandBuffer>,
}

impl<B: Backend> Context<B> {
    pub fn build<W: HasRawWindowHandle>(window: &W, name: &str) -> Result<Self, &'static str> {
        let mut context = Self::from_window(window, name)?;
        context.add_device()?;
        context.add_swapchain(0)?;
        context.add_semaphores(0, 0)?;
        context.devices[0].add_render_pass()?;
        context.devices[0].add_image_views(0)?;
        context.devices[0].add_framebuffers(0, 0)?;

        context.command_pools.push(unsafe {
            context.devices[0]
                .device
                .create_command_pool(
                    context.devices[0].queue.family,
                    CommandPoolCreateFlags::RESET_INDIVIDUAL,
                )
                .map_err(|_| "Could not create the raw command pool!")?
        });

        context.command_buffers =
            context.devices[0].create_command_buffers(&mut context.command_pools[0]);

        Ok(context)
    }

    pub fn from_window<W: HasRawWindowHandle>(
        window: &W,
        name: &str,
    ) -> Result<Self, &'static str> {
        let raw_instance =
            B::Instance::create(name, 1).map_err(|_| "failed to create the instance")?;

        let surface = unsafe {
            raw_instance
                .create_surface(window)
                .map_err(|_| "failed to create the surface")?
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
            surface,
            adapters,
            devices: vec![],
            command_pools: vec![],
            command_buffers: vec![],
        })
    }

    fn add_device(&mut self) -> Result<(), &'static str> {
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
            .ok_or("Failed to find a working queue or something")?;
        // TODO: Make this good
        let queue_group = queue_groups
            .into_iter()
            .next()
            .ok_or("Couldn't take ownership of the QueueGroup")?;

        if queue_group.queues.is_empty() {
            return Err("The QueueGroup did not have any CommandQueues available!");
        };

        self.devices
            .push(DeviceData::from(index, device, queue_group));

        Ok(())
    }

    fn add_swapchain(&mut self, device_index: usize) -> Result<(), &'static str> {
        let DeviceData {
            adapter_index,
            device,
            ..
        } = self
            .devices
            .get(device_index)
            .ok_or("Failed to get device")?;

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
            .ok_or("No PresentMode values specified!")?
        };

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
                None => formats
                    .get(0)
                    .cloned()
                    .ok_or("Preferred format list was empty!")?,
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
                .map_err(|_| "Failed to create the swapchain!")?
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

    fn add_semaphores(
        &mut self,
        device_index: usize,
        swapchain_index: usize,
    ) -> Result<(), &'static str> {
        self.devices
            .get_mut(device_index)
            .ok_or("No device with this index")?
            .add_semaphores(swapchain_index)
    }

    pub fn clear(&mut self, color: [f32; 4]) -> Result<(), &'static str> {
        self.devices[0].clear_frame(color, &mut self.command_buffers)
    }
}
