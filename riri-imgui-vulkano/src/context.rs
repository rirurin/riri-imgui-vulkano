use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use riri_mod_tools_rt::logln;
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo};
use vulkano::device::physical::PhysicalDevice;
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::swapchain::Surface;
use vulkano::{Version, VulkanLibrary};
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::Window;
use crate::device_select::DeviceSelector;
use crate::error::{LibError, Result};
use crate::resources::{HasLogicalDevice, HasPhysicalDevice, HasQueue, HasStandardMemoryAllocator, HasSurface};

#[derive(Debug)]
pub struct RendererContext {
    surface: Arc<Surface>,
    physical_device: Arc<PhysicalDevice>,
    logical_device: Arc<Device>,
    queue: Arc<Queue>,
    allocator: Arc<StandardMemoryAllocator>,
    #[allow(dead_code)]
    debug_messenger: DebugUtilsMessenger
}

impl RendererContext {
    pub fn new<D: HasDisplayHandle>(
        display_handle: D,
        window: Arc<Box<dyn Window>>,
        app_name: Option<String>
    ) -> Result<Self> {
        let start = Instant::now();
        // Initialize library
        let library = VulkanLibrary::new()?;
        let max_version = library.api_version();
        let mut required_extensions = Surface::required_extensions(&display_handle)?;
        required_extensions.ext_debug_utils = true;
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                application_name: app_name,
                application_version: Version::from_str(crate::version::RELOADED_VERSION)?,
                enabled_extensions: required_extensions,
                max_api_version: Some(max_version),
                ..Default::default()
            }
        )?;
        let mut debug_messenger_creation
            = DebugUtilsMessengerCreateInfo::user_callback(unsafe {
            DebugUtilsMessengerCallback::new(|flag, ty, data| {
                if flag.contains(DebugUtilsMessageSeverity::ERROR) {
                    logln!(Error, "{:?}: {}", ty, data.message);
                } else if flag.contains(DebugUtilsMessageSeverity::WARNING) {
                    logln!(Warning, "{:?}: {}", ty, data.message);
                } else if flag.contains(DebugUtilsMessageSeverity::INFO) {
                    logln!(Information, "{:?}: {}", ty, data.message);
                } else {
                    logln!(Debug, "{:?}: {}", ty, data.message);
                };
            })
        });
        debug_messenger_creation.message_severity = DebugUtilsMessageSeverity::INFO |
            DebugUtilsMessageSeverity::WARNING |
            DebugUtilsMessageSeverity::ERROR;
        debug_messenger_creation.message_type = DebugUtilsMessageType::GENERAL |
            DebugUtilsMessageType::PERFORMANCE |
            DebugUtilsMessageType::VALIDATION;
        let debug_messenger = DebugUtilsMessenger::new(instance.clone(), debug_messenger_creation)?;
        let surface = Surface::from_window(instance.clone(), window.clone())?;
        let device_selector = DeviceSelector::new(instance.clone(), surface.clone());
        let (physical_device, queue_family_index)
            = device_selector.select_best_device()?;
        let (logical_device, mut queues)
            = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![
                    QueueCreateInfo {
                        queue_family_index,
                        ..Default::default()
                    }
                ],
                enabled_extensions: device_selector.required_device_extensions,
                ..Default::default()
            }
        )?;
        let queue = queues.next().ok_or(LibError::NoGraphicsQueue)?;
        let allocator = Arc::new(
            StandardMemoryAllocator::new_default(logical_device.clone()));
        let time_ms = Instant::now().duration_since(start).as_micros() as f64 / 1000.;
        logln!(Information, "VulkanInit::new: Completed in {} ms", time_ms);
        Ok(Self {
            surface,
            physical_device,
            logical_device,
            queue,
            allocator,
            debug_messenger
        })
    }
}

impl HasLogicalDevice for RendererContext {
    fn logical_device(&self) -> Arc<Device> {
        self.logical_device.clone()
    }
}

impl HasSurface for RendererContext {
    fn surface(&self) -> Arc<Surface> {
        self.surface.clone()
    }
}

impl HasPhysicalDevice for RendererContext {
    fn physical_device(&self) -> Arc<PhysicalDevice> {
        self.physical_device.clone()
    }
}

impl HasQueue for RendererContext {
    fn queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }
}

impl HasStandardMemoryAllocator for RendererContext {
    fn allocator(&self) -> Arc<StandardMemoryAllocator> {
        self.allocator.clone()
    }
}