use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::pipeline::{ComputePipeline};
use vulkano::sync::GpuFuture;
use vulkano::VulkanLibrary;
use gpu_allocator::vulkan::Allocator;
use std::sync::Arc;

pub fn init_vulkan() -> Result<Arc<Instance>, Box<dyn std::error::Error>> {
    let library = VulkanLibrary::new()?;
    let instance = Instance::new(library, InstanceCreateInfo::default())?;
    Ok(instance)
}

pub async fn run_compute_shader(_instance: &Arc<Instance>, input: &str) -> String {
    format!("GPU processed: {}", input)
}
