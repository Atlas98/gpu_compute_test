RWStructuredBuffer<float> array_buffer;

[shader("compute")]
[numthreads(1,1,1)]
void computeMain(uint3 threadId : SV_DispatchThreadID)
{
    uint index = threadId.x;
    array_buffer[index] += 1;
}
