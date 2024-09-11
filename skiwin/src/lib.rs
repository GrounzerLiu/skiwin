pub mod cpu;
#[cfg(feature = "vulkan")]
pub mod vulkan;
#[cfg(feature = "gl")]
pub mod gl;

use std::ops::Deref;
use skia_safe::gpu::{Budgeted, DirectContext, SurfaceOrigin};
use skia_safe::{ImageInfo, Surface};
use softbuffer::SoftBufferError;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub trait SkiaWindow: Deref<Target = Window> {
    // fn resumed(&mut self);
    fn resize(&mut self) -> Result<(), SoftBufferError>;
    fn surface(&mut self) -> &mut Surface;
    fn present(&mut self);
}

pub(crate) fn create_surface(skia_context: &mut DirectContext, size: impl Into<PhysicalSize<u32>>) -> Surface {
    let size = size.into();
    let width = size.width;
    let height = size.height;
    let image_info = ImageInfo::new_n32_premul((width as i32, height as i32), None);
    skia_safe::gpu::surfaces::render_target(
        skia_context,
        Budgeted::Yes,
        &image_info,
        None,
        SurfaceOrigin::TopLeft,
        None,
        false,
        None,
    ).unwrap()
}