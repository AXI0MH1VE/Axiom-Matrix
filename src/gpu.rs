use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::pipeline::{ComputePipeline};
use vulkano::sync::GpuFuture;
use vulkano::VulkanLibrary;
use gpu_allocator::vulkan::Allocator;
use std::sync::Arc;
use rust_gpu::spirv;

#[spirv(compute)]
pub fn rust_compute_shader(input: &str) -> String {
    input.to_string()
}

pub fn init_vulkan() -> Result<Arc<Instance>, Box<dyn std::error::Error>> {
    let library = VulkanLibrary::new()?;
    let instance = Instance::new(library, InstanceCreateInfo::default())?;
    Ok(instance)
}

pub async fn run_compute_shader(instance: &Arc<Instance>, input: &str) -> String {
    let device = Device::new(
        instance.clone(),
        instance.enumerate_physical_devices()?.next().unwrap(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo::default()],
            ..Default::default()
        },
    ).expect("Failed to create device");

    let queue = device.queues().next().unwrap();
    let allocator = Allocator::new(&device.physical_device(), &device).unwrap();

    let shader_module = rust_compute_shader::load(device.clone()).expect("Shader load fail");
    let pipeline = ComputePipeline::new(device.clone(), shader_module.entry_point("main").unwrap(), &(), None, |_| {}).expect("Pipeline fail");

    let input_bytes = input.as_bytes().to_vec();
    let buffer = Buffer::from_iter(
        device.clone(),
        BufferCreateInfo { usage: BufferUsage::STORAGE_BUFFER, ..Default::default() },
        input_bytes,
    ).expect("Buffer fail");

    let mut builder = AutoCommandBufferBuilder::primary(device.clone(), queue.queue_family_index(), CommandBufferUsage::OneTimeSubmit).expect("Cmd fail");
    builder.bind_pipeline_compute(pipeline).dispatch([1, 1, 1]).unwrap();
    let command_buffer = builder.build().unwrap();

    let future = vulkano::sync::now(device.clone()).then_execute(queue.clone(), command_buffer).unwrap().then_signal_fence_and_flush().unwrap();
    future.wait(None).expect("Exec fail");

    String::from_utf8(buffer.read().unwrap().to_vec()).unwrap_or(input.to_string())
}