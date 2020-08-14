#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
#[cfg(feature = "default")]
use gfx_backend_vulkan as back;

pub mod graphics;

pub type Renderer = graphics::Context<back::Backend>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
