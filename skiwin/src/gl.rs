use std::num::NonZeroU32;
use std::ops::Deref;
use glutin::config::{ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder, GlConfig};
use skia_safe::gpu::DirectContext;
use skia_safe::{ImageInfo, Surface};
use std::sync::Arc;
use winit::window::Window;
use glutin::display::{GetGlDisplay, GlDisplay};

#[cfg(target_os = "macos")]
use glutin::api::cgl::{device::Device, display::Display};
#[cfg(any(target_os = "android", target_os = "linux"))]
use glutin::api::egl::{device::Device, display::Display};
#[cfg(target_os = "windows")]
use glutin::api::wgl::{device::Device, display::Display};
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentGlContext};
use softbuffer::SoftBufferError;
use crate::{create_surface, SkiaWindow};

pub struct GlWindow {
    skia_context: DirectContext,
    skia_surface: Surface,
    soft_buffer_surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}

impl GlWindow {
    pub fn new(window: Window) -> Self {
        let devices = Device::query_devices().expect("Failed to query devices").collect::<Vec<_>>();

        for (index, device) in devices.iter().enumerate() {
            println!(
                "Device {}: Name: {} Vendor: {}",
                index,
                device.name().unwrap_or("UNKNOWN"),
                device.vendor().unwrap_or("UNKNOWN")
            );
        }

        let device = devices.first().expect("No available devices");

        // Create a display using the device.
        let display =
            unsafe { Display::with_device(device, None) }.expect("Failed to create display");

        let template = config_template();
        let config = unsafe { display.find_configs(template) }
            .unwrap()
            .reduce(
                |config, acc| {
                    if config.num_samples() > acc.num_samples() {
                        config
                    } else {
                        acc
                    }
                },
            )
            .expect("No available configs");

        println!("Picked a config with {} samples", config.num_samples());

        // Context creation.
        //
        // In particular, since we are doing offscreen rendering we have no raw window
        // handle to provide.
        let context_attributes = ContextAttributesBuilder::new().build(None);

        // Since glutin by default tries to create OpenGL core context, which may not be
        // present we should try gles.
        let fallback_context_attributes =
            ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(None)).build(None);

        let not_current = unsafe {
            display.create_context(&config, &context_attributes).unwrap_or_else(|_| {
                display
                    .create_context(&config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
        };

        // Make the context current for rendering
        let context = not_current.make_current_surfaceless().unwrap();
        println!("Context created: {:?}", context.is_current());


        let interface = skia_safe::gpu::gl::Interface::new_load_with_cstr(|name|{
            context.display().get_proc_address(name)
        }).unwrap();


        let mut skia_context = skia_safe::gpu::direct_contexts::make_gl(interface, None).unwrap();

        let window = Arc::new(window);
        let size = window.inner_size();
        let skia_surface = create_surface(&mut skia_context, size);
        let soft_buffer_context = softbuffer::Context::new(window.clone()).unwrap();
        let soft_buffer_surface = softbuffer::Surface::new(&soft_buffer_context, window.clone()).unwrap();
        Self {
            skia_context,
            skia_surface,
            soft_buffer_surface,
        }
    }
}

impl SkiaWindow for GlWindow {
    fn resize(&mut self) -> Result<(), SoftBufferError> {
        let size = self.soft_buffer_surface.window().inner_size();
        let width = NonZeroU32::new(size.width).unwrap();
        let height = NonZeroU32::new(size.height).unwrap();
        let result = self.soft_buffer_surface.resize(width, height);
        match result {
            Ok(_) => {
                self.skia_surface = create_surface(&mut self.skia_context, size);
                Ok(())
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    fn surface(&mut self) -> &mut Surface {
        &mut self.skia_surface
    }

    fn present(&mut self) {
        let size = self.soft_buffer_surface.window().inner_size();
        let mut soft_buffer = self.soft_buffer_surface.buffer_mut().unwrap();
        let u8_slice = bytemuck::cast_slice_mut::<u32, u8>(&mut soft_buffer);
        let image_info = ImageInfo::new_n32_premul((size.width as i32, size.height as i32), None);
        self.skia_surface.read_pixels(
            &image_info,
            u8_slice,
            size.width as usize * 4,
            (0, 0),
        );
        soft_buffer.present().unwrap();
    }
}

impl Deref for GlWindow {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        self.soft_buffer_surface.window()
    }
}

impl AsRef<Window> for GlWindow {
    fn as_ref(&self) -> &Window {
        self.soft_buffer_surface.window()
    }
}

fn config_template() -> ConfigTemplate {
    ConfigTemplateBuilder::default()
        .with_alpha_size(8)
        // Offscreen rendering has no support window surface support.
        .with_surface_type(ConfigSurfaceTypes::empty())
        .build()
}