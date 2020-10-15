use gfx_hal::{self as hal};
use std::error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum Error {
    InstanceCreationError(gfx_hal::UnsupportedBackend),
    SurfaceCreationError(gfx_hal::window::InitError),
    QueueGroupError(QueueGroupError),
    CommandPoolCreationError,
    FenceCreationError,
    SemaphoreCreationError,
    RenderPassCreationError,
    ImageViewCreationError,
    FramebufferCreationError,
    FenceError(FenceOp),
    SemaphoreError,
    SubmissionError,
    SwapchainError(SwapchainError),
    BufferError(BufferOp, BufferKind),
    MemoryError(MemoryError, MemoryKind),
    MissingDevice(usize),
    MissingSwapchain(usize),
    MissingCommandPool(usize),
    MissingResourceManager(usize),
    MissingPipeline(usize),
    ShaderCreation(ShaderKind, gfx_hal::device::ShaderError),
    DescriptorSetLayoutCreation,
    PipelineLayoutCreation,
    MissingDescriptorSetLayout,
    PipelineCreation,
    IOError(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Error::*;
        let s = match self {
            InstanceCreationError(_) => "Failed creating the instance".to_string(),
            SurfaceCreationError(_) => "Failed creating the surface".to_string(),
            QueueGroupError(e) => format!("QueueGroup error: {}", e),
            MissingDevice(idx) => format!("Couldn't find the device with index {}", idx),
            SwapchainError(_) => "Failed creating the swapchain".to_string(),
            CommandPoolCreationError => "Failed creating the command pool".to_string(),
            FenceCreationError => "Failed creating a fence".to_string(),
            SemaphoreCreationError => "Failed creating a semaphore".to_string(),
            RenderPassCreationError => "Failed creating a render pass".to_string(),
            ImageViewCreationError => "Failed creating an image view".to_string(),
            FramebufferCreationError => "Failed creating a framebuffer".to_string(),
            FenceError(op) => format!(
                "Failed trying to {} a fence",
                match op {
                    FenceOp::Wait => "wait on",
                    FenceOp::Reset => "reset",
                    FenceOp::Acquire => "acquire",
                }
            ),
            SemaphoreError => "Failed to get a semaphore".to_string(),
            SubmissionError => "Failed to submit".to_string(),
            BufferError(op, kind) => format!(
                "Failed to {} a {} buffer",
                match op {
                    BufferOp::Create(_) => "create",
                    BufferOp::Bind(_) => "bind",
                },
                match kind {
                    BufferKind::Instance => "instance",
                    BufferKind::Staging => "staging",
                    BufferKind::Index => "index",
                    BufferKind::Matrix => "matrix",
                    BufferKind::Quad => "quad",
                    BufferKind::Image => "image",
                }
            ),
            MemoryError(err_kind, mem_kind) => {
                let mem_kind = match mem_kind {
                    MemoryKind::Staging => "staging",
                    MemoryKind::Geometry => "geometry",
                    MemoryKind::Index => "index",
                    MemoryKind::Image => "image",
                };
                match err_kind {
                    crate::error::MemoryError::AllocationError(_) => {
                        format!("Failed to allocate {} memory", mem_kind)
                    }
                    crate::error::MemoryError::NoSupportedMemory => {
                        format!("Found no supporting {} memory", mem_kind)
                    }
                    crate::error::MemoryError::MappingError => {
                        format!("Failed to map {} memory", mem_kind)
                    }
                }
            }
            MissingSwapchain(idx) => format!("Failed to retreive swapchain at index {}", 0),
            MissingCommandPool(idx) => format!("Failed to retreive command pool at index {}", 0),
            MissingResourceManager(idx) => {
                format!("Failed to retreive resource manager at index {}", 0)
            }
            MissingPipeline(idx) => format!("Failed to retreive graphics pipeline at index {}", 0),
            PipelineCreation => "Failed to create pipeline".to_string(),
            DescriptorSetLayoutCreation => "Failed to create descriptor set layout".to_string(),
            PipelineLayoutCreation => "Failed to create pipeline layout".to_string(),
            MissingDescriptorSetLayout => {
                "Missing descriptor set trying to add pipeline layout".to_string()
            }
            ShaderCreation(kind, e) => format!(
                "Failed to create {} shader ({})",
                match kind {
                    ShaderKind::Vertex => "vertex",
                    ShaderKind::Fragment => "fragment",
                },
                e
            ),
            IOError(e) => format!("IO error: {}", e),
        };

        write!(f, "{}", s)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::SurfaceCreationError(e) => Some(e),
            Error::BufferError(BufferOp::Create(e), _) => Some(e),
            Error::BufferError(BufferOp::Bind(e), _) => Some(e),
            Error::MemoryError(MemoryError::AllocationError(e), _) => Some(e),
            Error::ShaderCreation(_, e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ShaderKind {
    Vertex,
    Fragment,
}

#[derive(Debug)]
pub enum FenceOp {
    Reset,
    Acquire,
    Wait,
}

#[derive(Debug)]
pub enum MemoryError {
    AllocationError(hal::device::AllocationError),
    NoSupportedMemory,
    MappingError,
}

#[derive(Debug, Copy, Clone)]
pub enum MemoryKind {
    Staging,
    Geometry,
    Index,
    Image,
}

#[derive(Debug, Copy, Clone)]
pub enum BufferKind {
    Instance,
    Quad,
    Matrix,
    Staging,
    Image,
    Index,
}

#[derive(Debug)]
pub enum BufferOp {
    Bind(hal::device::BindError),
    Create(hal::buffer::CreationError),
}

#[derive(Debug)]
pub enum SwapchainError {
    NoImageViews,
    ImageAcquireError,
    NoPresentMode,
    CreationError(gfx_hal::window::CreationError),
}

#[derive(Debug)]
pub enum QueueGroupError {
    QueueGroupNotFound,
    OwnershipFailed,
    NoCommandQueues,
}

impl Display for QueueGroupError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(
            f,
            "{}",
            match self {
                QueueGroupError::QueueGroupNotFound => "Couldn't find an applicable QueueGroup",
                QueueGroupError::OwnershipFailed => "Couldn't take ownership of the QueueGroup",
                QueueGroupError::NoCommandQueues => "The QueueGroup didn't have any CommandQueues",
            }
        )
    }
}
