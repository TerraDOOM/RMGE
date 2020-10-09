#[derive(Debug)]
pub struct Error {
    pub description: String,
    pub error_kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    InstanceCreationError,
    SurfaceCreationError,
    QueueGroupError,
    DeviceNotFoundError,
    SwapchainCreationError,
    CommandPoolCreationError,
    FenceCreationError,
    SemaphoreCreationError,
    RenderPassCreationError,
    ImageViewCreationError,
    FramebufferCreationError,
    FenceError,
    SemaphoreError,
    SubmissionError,
    SwapchainError,
}
