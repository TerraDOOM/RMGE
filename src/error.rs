use gfx_hal::device;
use std::error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum Error {
    InstanceCreationError(gfx_hal::UnsupportedBackend),
    SurfaceCreationError(gfx_hal::window::InitError),
    QueueGroupError(QueueGroupError),
    DeviceNotFoundError(usize),
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
    MissingSwapchain,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Error::*;
        let s = match self {
            InstanceCreationError(e) => "Failed creating the instance".to_string(),
            SurfaceCreationError(e) => "Failed creating the surface".to_string(),
            QueueGroupError(e) => format!("QueueGroup error: {}", e),
            DeviceNotFoundError(idx) => format!("Couldn't find the device with index {}", idx),
            SwapchainError(e) => "Failed creating the swapchain".to_string(),
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
                    BufferOp::Create => "create",
                    BufferOp::Bind => "bind",
                },
                match kind {
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
                    crate::error::MemoryError::AllocationError => {
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
            MissingSwapchain => "No swapchain found".to_string(),
        };

        writeln!(f, "{}", s)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::SurfaceCreationError(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub enum FenceOp {
    Reset,
    Acquire,
    Wait,
}

#[derive(Debug)]
pub enum MemoryError {
    AllocationError,
    NoSupportedMemory,
    MappingError,
}

#[derive(Debug)]
pub enum MemoryKind {
    Staging,
    Geometry,
    Index,
    Image,
}

#[derive(Debug)]
pub enum BufferKind {
    Quad,
    Matrix,
    Staging,
    Image,
    Index,
}

#[derive(Debug)]
pub enum BufferOp {
    Bind,
    Create,
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
