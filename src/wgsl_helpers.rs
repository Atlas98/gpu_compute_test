 use wgpu::*;

pub fn create_compute_pipeline(device: &Device, pipeline_name: &str, shader_file: &str, entry: &str) -> ComputePipeline {
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
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    };
    device.create_buffer(&descriptor)
}

pub fn create_mapped_buffer(device: &Device, label: &str, size: u64) -> Buffer {
    let descriptor = BufferDescriptor {
        label: Some(label),
        size: size,
        usage: BufferUsages::MAP_WRITE | BufferUsages::MAP_READ | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    };
    device.create_buffer(&descriptor)
}

pub fn create_bindings_from_arrays(device: &Device, pipeline: &ComputePipeline, label: &str, arrays: &[&Buffer]) -> BindGroup {
    let mut bind_groups = Vec::new();
    for (index, buffer) in arrays.iter().enumerate() {
        let bind_group_entry = BindGroupEntry {
            binding: index as u32,
            resource: buffer.as_entire_binding()
        };
        bind_groups.push(bind_group_entry);
    }

    let bind_group_layout = pipeline.get_bind_group_layout(0);
    
    device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout: &bind_group_layout,
        entries: &bind_groups
    })
}


pub async fn request_gpu_resource() -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {

    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");

    // Get adapter limits and increase storage buffer binding size
    let mut limits = adapter.limits();
    limits.max_storage_buffer_binding_size = 512 * 1024 * 1024; // 512 MB for storage

    println!("Adapter chosen: {}", adapter.get_info().name);
    if adapter.features().contains(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS) {
        println!("MAPPABLE_PRIMARY_BUFFERS is supported.");
    } else {
        println!("MAPPABLE_PRIMARY_BUFFERS is not supported.");
    }

    let mut features = wgpu::Features::empty();
    features.set(Features::MAPPABLE_PRIMARY_BUFFERS, true);
    features.set(Features::TIMESTAMP_QUERY_INSIDE_ENCODERS, true);
    features.set(Features::TIMESTAMP_QUERY_INSIDE_PASSES, true);
    features.set(Features::TIMESTAMP_QUERY, true);

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