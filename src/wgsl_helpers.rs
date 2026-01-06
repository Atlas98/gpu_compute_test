   //let staging_buffer = _device.create_buffer(&BufferDescriptor { 
   //    label: Some("Persistent staging buffer"),
   //    size: total_size as u64 * std::mem::size_of::<u32>() as u64,
   //    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
   //    mapped_at_creation: false
   //});
   //
   //let upload_buffer = _device.create_buffer(&BufferDescriptor {
   //    label: Some("Persistent upload buffer"),
   //    size: total_size as u64 * std::mem::size_of::<u32>() as u64,
   //    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
   //    mapped_at_creation: true
   //});
   //
   //let shader_module = _device.create_shader_module(wgpu::ShaderModuleDescriptor {
   //    label: Some("Sort shader"),
   //    source: wgpu::ShaderSource::Wgsl(include_str!("sort.wgsl").into()),
   //});
   //println!("Created shader module");
   //let pipeline = _device.create_compute_pipeline(&ComputePipelineDescriptor {
   //    label: Some("Sort pipeline"),
   //    layout: None,
   //    entry_point: Some("main"),
   //    module: &shader_module,
   //    cache: None,
   //    compilation_options: PipelineCompilationOptions::default(),
   //});
     
use wgpu::*;

pub fn create_compute_pipeline(device: &wgpu::Device, pipeline_name: &str, shader_file: &str, entry: &str) -> ComputePipeline {
    // Read the shader file into a string at runtime
    let shader_source = std::fs::read_to_string(format!("shaders/{}", shader_file)).expect("Failed to read shader file");

    // Create the shader module from the loaded shader source code
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(shader_file),
        source: ShaderSource::Wgsl(shader_source.into()),
    });

    // Create the compute pipeline
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some(pipeline_name),
        layout: None,
        entry_point: Some(entry),
        module: &shader_module,
        cache: None,
        compilation_options: PipelineCompilationOptions::default(),
    });

    pipeline
}

pub fn create_storage_buffer(device: &wgpu::Device, label: &str, size: u64) -> Buffer {
    let descriptor = BufferDescriptor {
        label: Some(label),
        size: size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    };
    device.create_buffer(&descriptor)
}

pub fn create_upload_buffer(device: &wgpu::Device, label: &str, size: u64) -> Buffer {
    let descriptor = BufferDescriptor {
        label: Some(label),
        size: size,
        usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    };
    device.create_buffer(&descriptor)
}

pub fn create_download_buffer(device: &wgpu::Device, label: &str, size: u64) -> Buffer {
    let descriptor = BufferDescriptor {
        label: Some(label),
        size: size,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    };
    device.create_buffer(&descriptor)
}


pub async fn request_gpu_resource() -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {

    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");

    // Get adapter limits and increase storage buffer binding size
    let mut limits = adapter.limits();
    limits.max_storage_buffer_binding_size = 512 * 1024 * 1024; // 512 MB for storage

    if adapter.features().contains(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS) {
        println!("MAPPABLE_PRIMARY_BUFFERS is supported.");
    } else {
        println!("MAPPABLE_PRIMARY_BUFFERS is not supported.");
    }

    let mut features = wgpu::Features::empty();
    features.set(Features::MAPPABLE_PRIMARY_BUFFERS, true);

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            required_limits: limits,
            ..Default::default()
        })
        .await
        .expect("Failed to create device");

    let features = device.features();
    for feature in features.iter() {
        println!("Feature: {}", feature);
    }

    (adapter, device, queue)
}