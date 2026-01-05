use std::time::Instant;

#[tokio::main]
async fn main() {
 
    // CPU sorting
    println!("\nCPU Sorting:");
    //cpu_sort(num_arrays, array_size);
    
    path_entities().await;

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
    adapter.features().set(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS, true);

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Failed to create device");
    (adapter, device, queue)
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Transform {
    x: f32,
    y: f32,
    z: f32,
}

async fn path_entities() { 
    let (_adapter, device, queue) = request_gpu_resource().await;
    //let fence = device.create_fence(&wgpu::FenceDescriptor { label: None });
    
    let timer = Instant::now();
    let mut positions: Vec<Transform> = vec![];
    for x in 0..100 {
        for y in 0..100 {
            for z in 0..100 {
               positions.push(Transform {
                    x: x as f32,
                    y: y as f32,
                    z: z as f32
               }); 
            }
        }
    }
    println!("Time to create positions vector: {} ms", timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now();
    let buffer_size = (positions.len() * std::mem::size_of::<Transform>()) as u64;
    
    // Create upload buffer with mapped_at_creation for fastest initial data transfer
    let upload_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Upload Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
        mapped_at_creation: true,
    });
    
    // Write data directly via shared memory mapping (faster than CPU→GPU copy)
    {
        let mut mapped_data = upload_buffer.slice(..).get_mapped_range_mut();
        mapped_data.copy_from_slice(bytemuck::cast_slice(&positions));
    }
    upload_buffer.unmap();
    
    // Create GPU storage buffer for compute operations
    let a_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Positions buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // Queue immediate copy from upload buffer to storage buffer
    let mut copy_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    copy_encoder.copy_buffer_to_buffer(&upload_buffer, 0, &a_buffer, 0, buffer_size);
    queue.submit(Some(copy_encoder.finish()));
    device.poll(wgpu::Maintain::Wait);
    
    // Create read-back buffer for results
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    println!("Time to initialize buffer: {} ms", timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now();
    let shader_src = r#"
        // Define the Transform struct in the shader
        struct Transform {
            x: f32,
            y: f32,
            z: f32,
        };

        @group(0) @binding(0)
        var<storage, read_write> transforms: array<Transform>; // Array of Transform structs

        @compute @workgroup_size(256)
        fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
            let idx = global_id.x;
            if (idx < arrayLength(&transforms)) {
                // Add 1.0 to each component of the transform
                transforms[idx].x = transforms[idx].x + 1.0;
                transforms[idx].y = transforms[idx].y + 1.0;
                transforms[idx].z = transforms[idx].z + 1.0;

                // Use a proper loop with correct syntax
                var i : u32 = 101u; // Initialize loop variable
                for (; i < 100u; i = i + 1u) { // Iterate 100 times
                    transforms[idx].x = transforms[idx].x + 1.0;
                    transforms[idx].y = transforms[idx].y + 1.0;
                    transforms[idx].z = transforms[idx].z + 1.0;
                }
            }
        }
    "#;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: None,
        module: &shader,
        entry_point: "main",
    });

    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: a_buffer.as_entire_binding(),
            }
        ],
    });
    println!("Pipeline creation: {:.3}ms", timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((positions.len() as u32 + 255) / 256, 1, 1);
    } 

    let mut encoder2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut cpass = encoder2.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((positions.len() as u32 + 255) / 256, 1, 1);
    }
    // Combine copy with second encoder to batch GPU operations
    encoder2.copy_buffer_to_buffer(
        &a_buffer,
        0,
        &staging_buffer,
        0,
        buffer_size,
    );

    let cmd = encoder.finish();
    let cmd2 = encoder2.finish();

    println!("Encoder creation: {:.3}ms", timer.elapsed().as_secs_f64() * 1000.0);

    let compute_start = Instant::now();
    queue.submit(Some(cmd)); 
    device.poll(wgpu::Maintain::Wait);

    println!(" Transform Addition (1) - GPU Time: {:.3}ms", compute_start.elapsed().as_secs_f64() * 1000.0);
    
    let compute_start = Instant::now();
    queue.submit(Some(cmd2));
    device.poll(wgpu::Maintain::Wait);

    println!(" Transform Addition (2) + Copy - GPU Time: {:.3}ms", compute_start.elapsed().as_secs_f64() * 1000.0);

    let compute_start = Instant::now();
    read_buffer_transform(&device, &queue, &staging_buffer, positions.len()).await;

    println!(" Buffer Readback - GPU Time: {:.3}ms", compute_start.elapsed().as_secs_f64() * 1000.0);


}

// Helper function to read buffer data
async fn read_buffer_transform(device: &wgpu::Device, _queue: &wgpu::Queue, buffer: &wgpu::Buffer, _size: usize) {
    let buffer_slice = buffer.slice(..);
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
        let _ = tx.send(v);
    });
    device.poll(wgpu::Maintain::Wait);

    if let Some(Ok(())) = rx.receive().await {
        let data = buffer_slice.get_mapped_range();
        let result: &[Transform] = bytemuck::cast_slice(&data);
        println!("First result: {} {} {}", result[0].x, result[0].y, result[0].z);
        
        drop(data);
        buffer.unmap();
    }
}
