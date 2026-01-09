struct ArrayInfo {
    uint32_t array_size;
    uint32_t num_arrays;
};

RWStructuredBuffer<uint32_t> array_buffer; // Global data (RW buffer)
ConstantBuffer<ArrayInfo> array_info;      // Constant data (metadata)

// Declare groupshared memory to store data locally within the workgroup
groupshared uint local_arrays[256][32]; // 1024 threads, each with an array of 32 uints

[shader("compute")]
[numthreads(256, 1, 1)]
void main(uint3 threadId : SV_DispatchThreadID)
{
    // Calculate the index in the global array to be accessed by the thread
    uint index = threadId.x * array_info.array_size;  // Each thread jumps by array_size

    // Copy data from the global array (array_buffer) into local workgroup memory (local_array)
    for (var i = 0; i < 32; i++) {
        local_arrays[threadId.x][i] = array_buffer[index + i];
    }

    // Perform computation on the workgroup-local data
    for (var i = 0u; i < array_info.array_size; i++) {
        for (var j = 0; j < 1; j++) {
            local_arrays[threadId.x][i] += 1;  // This increments each element of the local array 1000 times
        }
    }

    // Synchronize the workgroup threads to ensure all have completed the computation
    // This ensures that all threads in the workgroup are done writing to `local_array`
    GroupMemoryBarrierWithGroupSync();

    // Write back the results from local_array to the global buffer
    // Make sure we write back in the same order
    for (var j = 0; j < 32; j++) {
        array_buffer[index + j] = local_arrays[threadId.x][j];
    }
}
