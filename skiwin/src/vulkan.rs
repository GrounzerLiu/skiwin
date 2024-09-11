use crate::SkiaWindow;
use ash::vk;
use ash::vk::Handle;
use skia_safe::gpu::vk::{BackendContext, GetProcOf};
use skia_safe::gpu::{Budgeted, DirectContext, SurfaceOrigin};
use skia_safe::{ISize, ImageInfo, Surface};
use softbuffer::SoftBufferError;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::ptr;
use std::sync::Arc;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo, QueueFlags};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions};
use vulkano::{VulkanLibrary, VulkanObject};
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct VulkanSkiaWindow {
    skia_context: DirectContext,
    skia_surface: Surface,
    vk_graphics: VkGraphics,
    soft_buffer_context: softbuffer::Context<Arc<Window>>,
    soft_buffer_surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    size: ISize,
}

impl VulkanSkiaWindow {
    pub fn new(window: Window) -> Self {
        let vk_graphics = VkGraphics::new("skia-org");
        let mut skia_context = {
            let get_proc = |of| unsafe {
                match vk_graphics.get_proc(of) {
                    Some(f) => f as _,
                    None => {
                        println!("resolve of {} failed", of.name().to_str().unwrap());
                        ptr::null()
                    }
                }
            };

            let backend_context = unsafe {
                BackendContext::new(
                    vk_graphics.instance.handle().as_raw() as _,
                    vk_graphics.physical_device.handle().as_raw() as _,
                    vk_graphics.device.handle().as_raw() as _,
                    (
                        vk_graphics.queue_and_index.0.handle().as_raw() as _,
                        vk_graphics.queue_and_index.1,
                    ),
                    &get_proc,
                )
            };

            skia_safe::gpu::direct_contexts::make_vulkan(&backend_context, None).unwrap()
        };

        let window = Arc::new(window);
        let skia_surface = create_surface(&mut skia_context, window.inner_size());
        let soft_buffer_context = softbuffer::Context::new(window.clone()).unwrap();
        let soft_buffer_surface = softbuffer::Surface::new(&soft_buffer_context, window).unwrap();

        Self {
            skia_context,
            skia_surface,
            vk_graphics,
            soft_buffer_context,
            soft_buffer_surface,
            size: Default::default(),
        }
    }

}


fn create_surface(skia_context:&mut DirectContext, size: impl Into<PhysicalSize<u32>>) -> Surface {
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
    )
        .unwrap()
}

impl SkiaWindow for VulkanSkiaWindow {
    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), SoftBufferError> {
        let width = NonZeroU32::new(size.width).unwrap();
        let height = NonZeroU32::new(size.height).unwrap();
        let result = self.soft_buffer_surface.resize(width, height);
        match result {
            Ok(_) => {
                self.skia_surface = create_surface(&mut self.skia_context, size);
                self.size = ISize::new(size.width as i32, size.height as i32);
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
        let mut soft_buffer = self.soft_buffer_surface.buffer_mut().unwrap();
        let u8_slice = bytemuck::cast_slice_mut::<u32, u8>(&mut soft_buffer);
        let image_info = ImageInfo::new_n32_premul((self.size.width, self.size.height), None);
        self.skia_surface.read_pixels(
            &image_info,
            u8_slice,
            self.size.width as usize * 4,
            (0, 0),
        );
        soft_buffer.present().unwrap();
    }
}

impl Deref for VulkanSkiaWindow {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        self.soft_buffer_surface.window()
    }
}

impl AsRef<Window> for VulkanSkiaWindow {
    fn as_ref(&self) -> &Window {
        self.soft_buffer_surface.window()
    }
}

pub struct VkGraphics {
    pub vulkan_library: Arc<VulkanLibrary>,
    pub instance: Arc<Instance>,
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    pub queue_and_index: (Arc<Queue>, usize),
}

// most code copied from here: https://github.com/MaikKlein/ash/blob/master/examples/src/lib.rs
impl VkGraphics {
    pub fn new(app_name: &str) -> VkGraphics {
        let vulkan_library = VulkanLibrary::new().unwrap();

        let instance: Arc<Instance> = {
            let mut instance_extensions = InstanceExtensions::default();
            instance_extensions.khr_get_display_properties2 = true;
            instance_extensions.khr_portability_enumeration = true;

            let mut create_info = InstanceCreateInfo::default();
            create_info.engine_name = Some(app_name.to_string());
            create_info.engine_name = Some(app_name.to_string());
            create_info.enabled_extensions = instance_extensions;
            create_info.flags = InstanceCreateFlags::ENUMERATE_PORTABILITY;

            Instance::new(vulkan_library.clone(), create_info).unwrap()
        };

        let (physical_device, queue_family_index) = {
            let physical_devices = instance
                .enumerate_physical_devices().unwrap();

            physical_devices
                .map(|physical_device| {
                    physical_device.queue_family_properties()
                        .iter()
                        .enumerate()
                        .find_map(|(index, info)| {
                            let supports_graphic = info.queue_flags.contains(QueueFlags::GRAPHICS);
                            supports_graphic.then_some((physical_device.clone(), index))
                        })
                })
                .find_map(|v| {
                    if let Some((physical_device, queue_family_index)) = &v {
                        let properties = physical_device.properties();
                        println!("Found suitable device: {:?}, index: {}", properties.device_name, queue_family_index);
                    }
                    v
                }
                )
                .expect("Failed to find a suitable Vulkan device")
        };

        let (device, queues) = {
            let mut queue_create_info = QueueCreateInfo::default();
            queue_create_info.queue_family_index = queue_family_index as _;

            let mut device_create_info = DeviceCreateInfo::default();
            device_create_info.queue_create_infos = vec![queue_create_info];


            Device::new(physical_device.clone(), device_create_info).unwrap()
        };

        let queue_index = 0;
        let (_, queue) = queues.enumerate().nth(queue_index).unwrap();

        VkGraphics {
            vulkan_library,
            instance,
            physical_device,
            device,
            queue_and_index: (queue, 0),
        }
    }

    pub unsafe fn get_proc(&self, of: GetProcOf) -> Option<unsafe extern "system" fn()> {
        match of {
            GetProcOf::Instance(instance, name) => {
                let ash_instance = vk::Instance::from_raw(instance as _);
                self.vulkan_library.get_instance_proc_addr(ash_instance, name)
            }
            GetProcOf::Device(device, name) => {
                let ash_device = vk::Device::from_raw(device as _);
                (self.instance.fns().v1_0.get_device_proc_addr)(ash_device, name)
            }
        }
    }
}
