use std::sync::Arc;
use vulkano::device::{DeviceExtensions, QueueFlags};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::instance::Instance;
use vulkano::swapchain::Surface;
use crate::error::{LibError, Result};
use crate::resources::{HasInstance, HasSurface};

#[derive(Debug)]
pub struct DeviceSelector {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    pub(crate) required_queue_flags: QueueFlags,
    pub(crate) required_device_extensions: DeviceExtensions
}

impl DeviceSelector {
    /// Create a new device selector inteded to target a physical device capable of running
    /// a real-time GUI (does it have a swapchain?).
    pub fn new(instance: Arc<Instance>, surface: Arc<Surface>) -> Self {
        Self::new_with_flags(
            instance, surface, QueueFlags::GRAPHICS,
            DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::empty()
            })
    }

    pub fn new_with_flags(
        instance: Arc<Instance>,
        surface: Arc<Surface>,
        // should be QueueFlags::GRAPHICS
        required_queue_flags: QueueFlags,
        // for real-time GUIs, this should be set to support swapchains
        required_device_extensions: DeviceExtensions
    ) -> Self {
        Self {
            instance,
            surface,
            required_queue_flags,
            required_device_extensions
        }
    }

    pub fn get_all_suitable_devices(&self) -> Result<Vec<Arc<PhysicalDevice>>> {
        Ok(self.instance
            .enumerate_physical_devices()?
            .filter(|phys|
                phys.supported_extensions().contains(&self.required_device_extensions))
            .collect()
        )
    }

    /// Returns the selected physical device and it's best queue family index
    pub fn select_best_device(&self) -> Result<(Arc<PhysicalDevice>, u32)> {
        Ok(self.instance
            .enumerate_physical_devices()?
            .filter(|phys|
                phys.supported_extensions().contains(&self.required_device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    // Find the first first queue family that is suitable.
                    // If none is found, `None` is returned to `filter_map`,
                    // which disqualifies this physical device.
                    .position(|(i, queue)| {
                        queue.queue_flags.contains(self.required_queue_flags) &&
                            p.surface_support(i as u32, &self.surface).unwrap_or(false)
                    })
                    .map(|q| (p, q as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 4
            })
            .ok_or(LibError::NoSuitablePhysicalDevice)?)
    }
}

impl HasInstance for DeviceSelector {
    fn instance(&self) -> Arc<Instance> {
        self.instance.clone()
    }
}

impl HasSurface for DeviceSelector {
    fn surface(&self) -> Arc<Surface> {
        self.surface.clone()
    }
}