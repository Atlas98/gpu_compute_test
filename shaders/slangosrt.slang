struct ArrayInfo {
    uint32_t array_size; // Size of each array
    uint32_t num_arrays; // Number of arrays to process
};

RWStructuredBuffer<uint32_t> array_buffer;   // Global storage buffer (RW)
ConstantBuffer<ArrayInfo> array_info;        // Constant buffer with array metadata

// Declare shared local memory (1 array per thread in the workgroup)
groupshared uint local_arrays[256][32]; // Each thread gets a local array of size 32 (for 256 threads)

// The main compute shader function
[numthreads(256, 1, 1)]  // 256 threads in a workgroup
void main(uint3 threadId : SV_DispatchThreadID)
{
    // Compute the global index for the thread (this thread will access one element of the global array)
    uint array_index = threadId.x; // This thread works on array_index-th array (0 to 255)
    uint thread_id = threadId.x % 256;
    
    if (array_index >= array_info.num_arrays) {
        return;  // Out of bounds: exit the thread if the array_index exceeds num_arrays
    }

    uint index = array_index * array_info.array_size;  // Start index in the global buffer for the array

    // Load data from the global array into local shared memory for this thread
    for (uint i = 0; i < array_info.array_size; i++) {
        local_arrays[thread_id][i] = array_buffer[index + i];
    }

    // Perform computation (incrementing 1000 times for each element of the array)
    for (uint i = 0u; i < array_info.array_size; i++) {
        for (uint j = 0u; j < 1000; j++) {
            local_arrays[thread_id][i] += 1;
        }
    }

    // Synchronize the threads in the workgroup to ensure all threads have finished computation
    GroupMemoryBarrierWithGroupSync();

    // Write back the updated results from local memory to the global buffer
    for (uint i = 0u; i < array_info.array_size; i++) {
        array_buffer[index + i] = local_arrays[thread_id][i];
    }
}
