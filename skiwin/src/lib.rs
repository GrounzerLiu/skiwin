pub mod cpu;
#[cfg(feature = "vulkan")]
pub mod vulkan;
#[cfg(feature = "gl")]
pub mod gl;

use std::ops::Deref;
use skia_safe::Surface;
use softbuffer::SoftBufferError;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub trait SkiaWindow: Deref<Target = Window> {
    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), SoftBufferError>;
    fn surface(&mut self) -> &mut Surface;
    fn present(&mut self);
}