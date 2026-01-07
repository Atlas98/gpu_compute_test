use std::time::Instant;

use wgpu::{ComputePassDescriptor, PollType, util::{BufferInitDescriptor, DeviceExt}};

use crate::wgsl_helpers::{create_compute_pipeline, create_mapped_buffer, create_storage_buffer, request_gpu_resource};
mod wgsl_helpers;

#[tokio::main]
async fn main() {
 
    let (adapter, device, queue) = request_gpu_resource().await;
    let arrays = create_random_arrays(100000, 64); 

    // Create persistent staging buffer and upload buffer outside timing
    let total_size = arrays.len() * arrays[0].len() * size_of::<u32>();
    let total_size = total_size as u64;
   
    let staging_buffer = create_mapped_buffer(&device, "Staging buffer", total_size);
    let upload_buffer = create_mapped_buffer(&device, "Upload buffer", total_size); 
    let pipeline = create_compute_pipeline(&device, "Basic compute", "sort.wgsl", "main");
/*
    upload_buffer.map_async(wgpu::MapMode::Write, .., |result| {});
    staging_buffer.map_async(wgpu::MapMode::Write, .., |result| {});
    let _ = device.poll(PollType::wait_indefinitely()); 
    upload_buffer.unmap();
    staging_buffer.unmap();
*/
    let timer = Instant::now();
    sort_arrays_gpu(&arrays, &device, &queue, &staging_buffer, &upload_buffer, &pipeline).await;
    println!("Total GPU sorting time: {:?} ms", timer.elapsed().as_secs_f64() * 1000.0);
    
    let timer = Instant::now();
    sort_arrays_cpu(&arrays);
    println!("Total CPU sorting time: {:?} ms", timer.elapsed().as_secs_f64() * 1000.0);
}


pub async fn sort_arrays_gpu(arrays: &Vec<Vec<u32>>, device: &wgpu::Device, queue: &wgpu::Queue, staging_buffer: &wgpu::Buffer, upload_buffer: &wgpu::Buffer, pipeline: &wgpu::ComputePipeline) {
    let num_arrays = arrays.len();
    let array_size = arrays[0].len() as u32;
    let total_size = num_arrays * array_size as usize;
    
    let timer = Instant::now();
    // Create buffers without initial data for faster creation
    let array_buffer = create_storage_buffer(&device, "Array buffer", (total_size * size_of::<u32>()) as u64);
    println!("{} time is {} ms", "Creating array_buffer", timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now();
    // Create a single flat vector to hold all the data
    let flattened_data: Vec<u8> = arrays.iter()
        .flat_map(|arr| bytemuck::cast_slice(arr).iter().copied())  // Convert each array to a slice of bytes and flatten them
        .collect();  // Collect into a single vector
    println!("{} time is {} ms", "[Upload] Flatenning array" , timer.elapsed().as_secs_f64() * 1000.0);
    // Step 2: Write the flattened data to the GPU buffer
    //queue.write_buffer(&upload_buffer, 0, &flattened_data);

    let timer = Instant::now(); 
    upload_buffer.map_async(wgpu::MapMode::Write, .., |result| {});
    let _ = device.poll(PollType::wait_indefinitely());
    println!("{} time is {} ms", "[Upload] Mapping request duration" , timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now();
    upload_buffer.get_mapped_range_mut(..).copy_from_slice(bytemuck::cast_slice(&flattened_data));
    upload_buffer.unmap();

    let elapsed_seconds = timer.elapsed().as_secs_f64();
    let bytes = (total_size * size_of::<u32>() / 1024 / 1024) as f64;
    let throughput = bytes / elapsed_seconds;
    println!("{} time is {} ms, upload throughput = {} MB/s", "[Upload] Uploading buffer via copy_from_slice", elapsed_seconds * 1000.0, throughput);

    let timer = Instant::now(); 
    // Combine uniform data into single buffer to reduce buffer count
    let mut uniform_data = Vec::new();
    uniform_data.extend_from_slice(bytemuck::bytes_of(&array_size));
    uniform_data.extend_from_slice(bytemuck::bytes_of(&(num_arrays as u32)));
    
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor { 
        label: Some("Uniform buffer"),
        contents: &uniform_data,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    println!("{} time is {} ms", "Creating uniform_buffer", timer.elapsed().as_secs_f64() * 1000.0);
    

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
    println!("{} time is {} ms", "bind groups", timer.elapsed().as_secs_f64() * 1000.0);
    


    let timer = Instant::now();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Sort command encoder"),
    });
    encoder.copy_buffer_to_buffer(&upload_buffer, 0, &array_buffer, 0, total_size as u64 * std::mem::size_of::<u32>() as u64);
 

    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Some compute pass"),
            timestamp_writes: None
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups((num_arrays as u32 + 255) / 256, 1, 1);
    }
    // Copy from upload buffer to array buffer, then array buffer to staging buffer
   encoder.copy_buffer_to_buffer(&array_buffer, 0, &staging_buffer, 0, total_size as u64 * std::mem::size_of::<u32>() as u64);
    let command_buffer = encoder.finish();
    println!("{} time is {} ms", "Encoder + command buffers", timer.elapsed().as_secs_f64() * 1000.0);
    
    let timer = Instant::now();
    queue.submit(std::iter::once(command_buffer)); 
    let _ = device.poll(PollType::wait_indefinitely());

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
    let _ = device.poll(PollType::wait_indefinitely());

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