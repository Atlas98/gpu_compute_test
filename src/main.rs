use std::time::Instant;

use wgpu::{BufferDescriptor, ComputePassDescriptor, ComputePipelineDescriptor, util::{BufferInitDescriptor, DeviceExt}};

#[tokio::main]
async fn main() {
 
    let (_adapter, _device, _queue) = request_gpu_resource().await;
    let arrays = create_random_arrays(1000000, 32); 

    let timer = Instant::now();
    sort_arrays_cpu(&arrays);
    println!("Total CPU sorting time: {:?} ms", timer.elapsed().as_secs_f64() * 1000.0);

    // Create persistent staging buffer and upload buffer outside timing
    let total_size = arrays.len() * arrays[0].len();
    let staging_buffer = _device.create_buffer(&BufferDescriptor { 
        label: Some("Persistent staging buffer"),
        size: total_size as u64 * std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false
    });
    
    let upload_buffer = _device.create_buffer(&BufferDescriptor {
        label: Some("Persistent upload buffer"),
        size: total_size as u64 * std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
        mapped_at_creation: true
    });

    let timer = Instant::now();
    sort_arrays_gpu(&arrays, &_device, &_queue, &staging_buffer, &upload_buffer).await;
    println!("Total GPU sorting time: {:?} ms", timer.elapsed().as_secs_f64() * 1000.0);
}


pub async fn sort_arrays_gpu(arrays: &Vec<Vec<u32>>, device: &wgpu::Device, queue: &wgpu::Queue, staging_buffer: &wgpu::Buffer, upload_buffer: &wgpu::Buffer) {
    let timer = Instant::now();
    let num_arrays = arrays.len();
    let array_size = arrays[0].len() as u32;
    let total_size = num_arrays * array_size as usize;
    let mut flat: Vec<u32> = Vec::with_capacity(total_size);

    for arr in arrays {
        flat.extend_from_slice(arr);
    }
    println!("{} time is {} ms", "Flattening", timer.elapsed().as_secs_f64() * 1000.0);
 
    let timer = Instant::now();
    // Create buffers without initial data for faster creation
    let array_buffer = device.create_buffer(&BufferDescriptor { 
        label: Some("Array buffer"),
        size: total_size as u64 * std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false
    });
    
    // Write data to mapped upload buffer (no blocking)
    {
        let mut mapped = upload_buffer.slice(..).get_mapped_range_mut();
        let slice = bytemuck::cast_slice_mut(&mut mapped);
        slice.copy_from_slice(&flat);
        drop(mapped); // Release the mapping
    }
    upload_buffer.unmap();
    
    // Combine uniform data into single buffer to reduce buffer count
    let mut uniform_data = Vec::new();
    uniform_data.extend_from_slice(bytemuck::bytes_of(&array_size));
    uniform_data.extend_from_slice(bytemuck::bytes_of(&(num_arrays as u32)));
    
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor { 
        label: Some("Uniform buffer"),
        contents: &uniform_data,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    println!("{} time is {} ms", "Buffer creation", timer.elapsed().as_secs_f64() * 1000.0);
    
    let timer = Instant::now(); 
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Sort shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("sort.wgsl").into()),
    });
    println!("Created shader module");
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("Sort pipeline"),
        layout: None,
        entry_point: "main",
        module: &shader_module
    });

    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Sort bind group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: array_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });
    println!("{} time is {} ms", "Pipeline + bind groups", timer.elapsed().as_secs_f64() * 1000.0);
    
    let timer = Instant::now();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Sort command encoder"),
    });
    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Some compute pass"),
            timestamp_writes: None
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups((num_arrays as u32 + 15) / 16, 1, 1);
    }
    // Copy from upload buffer to array buffer, then array buffer to staging buffer
    encoder.copy_buffer_to_buffer(&upload_buffer, 0, &array_buffer, 0, total_size as u64 * std::mem::size_of::<u32>() as u64);
    encoder.copy_buffer_to_buffer(&array_buffer, 0, &staging_buffer, 0, total_size as u64 * std::mem::size_of::<u32>() as u64);
    let command_buffer = encoder.finish();
    println!("{} time is {} ms", "Encoder + command buffers", timer.elapsed().as_secs_f64() * 1000.0);
    
    let timer = Instant::now();
    queue.submit(std::iter::once(command_buffer)); 
    device.poll(wgpu::Maintain::Wait);
    let submission_time = timer.elapsed().as_secs_f64() * 1000.0;
    println!("GPU compute + copy time: {:?} ms", submission_time);
    
    let timer = Instant::now();
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, 
        move |v| {
            let _ = sender.send(v);
        }
    );
    device.poll(wgpu::Maintain::Wait);

    receiver.receive().await.unwrap().unwrap();
    let data = buffer_slice.get_mapped_range();
    
    // Avoid unnecessary vector copy - work with slice directly
    let data_slice: &[u32] = bytemuck::cast_slice(&data);
    println!("Data slice size: {}", data_slice.len());   
    println!("Sorted first array(GPU): {:?}", &data_slice[(num_arrays - 1) * array_size as usize..(num_arrays) * array_size as usize]);
    
    drop(data);
    staging_buffer.unmap();
    println!("{} time is {} ms", "Readback", timer.elapsed().as_secs_f64() * 1000.0);

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

pub fn sort_arrays_cpu(arrays: &Vec<Vec<u32>>) {
    let mut last_arr = arrays[0].clone();
    for(i, array) in arrays.iter().enumerate() {
        let mut arr = array.clone();

        arr.sort();
        if i == arrays.len() - 1 {
            last_arr = arr.clone();
        }

    }
    println!("Sorted array {} on CPU: {:?}", 0, last_arr);

}

pub fn create_random_arrays(num_arrays: usize, size: usize) -> Vec<Vec<u32>> {
    (0..num_arrays).map(|_| {
        use rand::Rng;
        let mut rng = rand::rng();
        (0..size).map(|_| rng.random_range(0..100)).collect()

    }).collect()
}