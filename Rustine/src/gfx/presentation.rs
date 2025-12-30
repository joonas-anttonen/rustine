use crate::gfx::PixelBuffer;

pub enum AcquireStatus<'a> {
    Success(&'a PixelBuffer),
    Timeout,
    OutOfDate,
    Error(crate::gfx::Status),
}

pub trait PresentationProvider {
    fn acquire(&self) -> AcquireStatus<'_>;
}

pub struct SharedImageProvider {
    output_frame: PixelBuffer,
}
impl SharedImageProvider {
    pub fn new(output_frame: PixelBuffer) -> Self {
        SharedImageProvider { output_frame }
    }
}

impl PresentationProvider for SharedImageProvider {
    fn acquire(&self) -> AcquireStatus<'_> {
        AcquireStatus::Success(&self.output_frame)
    }
}

pub struct SwapchainProvider {}

impl PresentationProvider for SwapchainProvider {
    fn acquire(&self) -> AcquireStatus<'_> {
        AcquireStatus::Timeout
    }
}
