@binding(0) @group(0) var<storage, read_write> array_buffer_0 : array<f32>;

@compute
@workgroup_size(1, 1, 1)
fn computeMain(@builtin(global_invocation_id) threadId_0 : vec3<u32>)
{
    array_buffer_0[threadId_0.x] = array_buffer_0[threadId_0.x] + 1.0f;
    return;
}

