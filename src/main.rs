use std::{default, mem::zeroed, thread::sleep, time::{Duration, Instant}};

use bytemuck::{Pod, Zeroable, bytes_of};
use pollster::block_on;
use tracy_client::Client;
use wgpu::{Buffer, ComputePassDescriptor, ComputePipeline, Device, PollType, Queue, util::{BufferInitDescriptor, DeviceExt}};

use crate::wgsl_helpers::{create_bindings_from_arrays, create_compute_pipeline, create_mapped_buffer, create_storage_buffer, request_gpu_resource};
mod wgsl_helpers;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ArrayInfo {
    array_size_0: u32, // aligned to 16 bytes
    num_arrays_0: u32, // aligned to 4 bytes
    _padding: [u8; 8],     // Add 8 bytes of padding
}


fn main() {
    let client: Client = Client::start();
    let (_adapter, device, queue) = block_on(request_gpu_resource());
    let mut arrays = create_random_arrays(2000000, 32); 

    // Create persistent staging buffer and upload buffer outside timing
    let total_size = arrays.len() * arrays[0].len() * size_of::<u32>();
    let total_size = total_size as u64;
   
    let staging_buffer = create_mapped_buffer(&device, "Staging buffer", total_size);
    let upload_buffer = create_mapped_buffer(&device, "Upload buffer", total_size); 
    let pipeline = create_compute_pipeline(&device, "Basic compute", "slangsort.wgsl", "main");

    upload_buffer.map_async(wgpu::MapMode::Write, .., |_result| {});
    staging_buffer.map_async(wgpu::MapMode::Write, .., |_result| {});
    let _ = device.poll(PollType::wait_indefinitely()); 
    upload_buffer.unmap();
    staging_buffer.unmap();

    sleep(Duration::from_millis(200));
    let timer = Instant::now();
    sort_arrays_gpu(&arrays, &device, &queue, &staging_buffer, &upload_buffer, &pipeline);
    println!("Total GPU sorting time: {:?} ms", timer.elapsed().as_secs_f64() * 1000.0);
    
    let timer = Instant::now();
    sort_arrays_cpu(&arrays);
    println!("Total CPU sorting time: {:?} ms", timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now();
    cpu_process(&mut arrays);
    println!("Total CPU adding time: {:?} ms", timer.elapsed().as_secs_f64() * 1000.0);
}


pub fn sort_arrays_gpu(arrays: &Vec<Vec<u32>>, device: &Device, queue: &Queue, staging_buffer: &Buffer, upload_buffer: &Buffer, pipeline: &ComputePipeline) {
    let _span = tracy_client::span!("Sort on GPU");
    
    let num_arrays = arrays.len();
    let array_size = arrays[0].len() as u32;
    let total_size = num_arrays * array_size as usize;
    
    let timer = Instant::now();
    // Create buffers without initial data for faster creation
    let array_buffer = create_storage_buffer(&device, "Array buffer", (total_size * size_of::<u32>()) as u64);
    println!("{} time is {} ms", "Creating array_buffer", timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now(); 
    upload_buffer.map_async(wgpu::MapMode::Write, .., |_result| {});
    let _ = device.poll(PollType::wait_indefinitely());
    println!("{} time is {} ms", "[Upload] Mapping request duration" , timer.elapsed().as_secs_f64() * 1000.0);

    let timer = Instant::now();
    let mut offset = 0; // Keeps track of the current position in the buffer view
    let mut buffer_view = upload_buffer.get_mapped_range_mut(..);

    arrays.iter().for_each(|arr| {
        let slice = bytemuck::cast_slice(&arr);  // Convert the array to a byte slice
        let slice_len = slice.len();            // Get the length of the slice
        
        // Copy the slice data into the appropriate section of the buffer view
        buffer_view[offset..offset + slice_len].copy_from_slice(slice); 
        offset += slice_len;
    });
    drop(buffer_view);
    upload_buffer.unmap();

    let elapsed_seconds = timer.elapsed().as_secs_f64();
    let throughput = ((total_size * size_of::<u32>() / 1024 / 1024) as f64) / elapsed_seconds;
    println!("{} time is {} ms, upload throughput = {} MB/s", "[Upload] Uploading buffer via copy_from_slice", elapsed_seconds * 1000.0, throughput);

    let timer = Instant::now(); 
    // Combine uniform data into single buffer to reduce buffer count
    let mut uniform_data = Vec::new();
    uniform_data.extend_from_slice(bytes_of(&ArrayInfo {
        array_size_0: array_size,
        num_arrays_0: num_arrays as u32,
        _padding: [0,0,0,0,0,0,0,0],
    }));
    //uniform_data.extend_from_slice(bytemuck::bytes_of(&array_size));
    //uniform_data.extend_from_slice(bytemuck::bytes_of(&(num_arrays as u32)));
    
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor { 
        label: Some("Uniform buffer"),
        contents: &uniform_data,
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group = create_bindings_from_arrays(&device, &pipeline, "Basic bind group", &[&array_buffer, &uniform_buffer]);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Sort command encoder"),
    });

    //encoder.copy_buffer_to_buffer(&upload_buffer, 0, &array_buffer, 0, total_size as u64 * std::mem::size_of::<u32>() as u64); 
    encoder.insert_debug_marker("Before compute pass");
    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Some compute pass"),
            timestamp_writes: None
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups((num_arrays as u32 + 1023) / 1024, 1, 1);
    }
    encoder.insert_debug_marker("After compute pass");
    // Copy from upload buffer to array buffer, then array buffer to staging buffer
    encoder.copy_buffer_to_buffer(&array_buffer, 0, &staging_buffer, 0, total_size as u64 * std::mem::size_of::<u32>() as u64);
    let command_buffer = encoder.finish();
    println!("{} time is {} ms", "Uniforms + bind groups + encoder + commands", timer.elapsed().as_secs_f64() * 1000.0);
    
    let timer = Instant::now();
    queue.submit(std::iter::once(command_buffer)); 
    let _ = device.poll(PollType::wait_indefinitely());

   let submission_time = timer.elapsed().as_secs_f64() * 1000.0;
    println!("GPU compute + copy time: {:?} ms", submission_time);
    
    let timer = Instant::now();
    let buffer_slice = staging_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, move |_v| {});
    let _ = device.poll(PollType::wait_indefinitely());

    let data = buffer_slice.get_mapped_range();
    // Assuming `data_slice` is a reference to your flattened data buffer

    let data_slice: &[u32] = bytemuck::cast_slice(&data);
    println!("Sorted first array(GPU): {:?}", &data_slice[0 as usize..array_size as usize]);
    println!("Sorted first array(GPU): {:?}", &data_slice[(num_arrays - 1) * array_size as usize..(num_arrays) * array_size as usize]);

// Access the uniform variables
    // Loop through all arrays and print each one
    for _ in 0..num_arrays {
        //let start_index = i * array_size as usize;
        //let end_index = (i + 1) * array_size as usize;
        
        // Print the current array (i-th array)
        //println!("Sorted array {} (GPU): {:?}", i, &data_slice[start_index..end_index]);
    } 
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


pub fn cpu_process(data: &mut Vec<Vec<u32>>) {
    for array in data.iter_mut() {
        let array_size = array.len();

        for i in 0..array_size {
            for _j in 0..1 {
                let initial_value = array[i];
                let mut final_value = 0u32;

                if initial_value < 10 {
                    final_value = 5;
                }
                if initial_value > 10 && initial_value < 20 {
                    final_value = 15;
                }
                if initial_value > 20 && initial_value < 30 {
                    final_value = 25;
                }
                if initial_value > 30 && initial_value < 40 {
                    final_value = 35;
                }
                if initial_value > 40 && initial_value < 50 {
                    final_value = 45;
                }
                if initial_value > 50 && initial_value < 60 {
                    final_value = 55;
                }
                if initial_value > 60 && initial_value < 70 {
                    final_value = 65;
                }
                if initial_value > 70 && initial_value < 80 {
                    final_value = 75;
                }
                if initial_value > 80 && initial_value < 90 {
                    final_value = 85;
                }
                if initial_value > 90 && initial_value < 100 {
                    final_value = 95;
                }

                array[i] = final_value;
            }
        }
    }
}

pub fn create_random_arrays(num_arrays: usize, size: usize) -> Vec<Vec<u32>> {
    (0..num_arrays).map(|_| {
        use rand::Rng;
        let mut rng = rand::rng();
        (0..size).map(|_| rng.random_range(0..100)).collect()

    }).collect()
}