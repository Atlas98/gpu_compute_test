
struct ArrayInfo {
    uint32_t array_size;
    uint32_t num_arrays;
}

RWStructuredBuffer<uint32_t> array_buffer;
ConstantBuffer<ArrayInfo> array_info;


[shader("compute")]
[numthreads(1024,1,1)]
void main(uint3 threadId : SV_DispatchThreadID)
{
    uint index = threadId.x * array_info.array_size; // each thread jumps
    //if (index > array_info.num_arrays) return;

    for (var i = 0u; i < array_info.array_size; i++) {
        for (var j = 0; j < 1000; j++) {
            array_buffer[index + i] += 1;
        }
        
    }
}
