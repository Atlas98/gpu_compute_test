struct ArrayInfo_std140_0
{
    @align(16) array_size_0 : u32,
    @align(4) num_arrays_0 : u32,
};

@binding(1) @group(0) var<uniform> array_info_0 : ArrayInfo_std140_0;
@binding(0) @group(0) var<storage, read_write> array_buffer_0 : array<u32>;

@compute
@workgroup_size(1024, 1, 1)
fn main(@builtin(global_invocation_id) threadId_0 : vec3<u32>)
{
    var _S1 : u32 = threadId_0.x * array_info_0.array_size_0;
    var i_0 : u32 = u32(0);
    for(;;)
    {
        if(i_0 < (array_info_0.array_size_0))
        {
        }
        else
        {
            break;
        }
        var j_0 : i32 = i32(0);
        for(;;)
        {
            if(j_0 < i32(1000))
            {
            }
            else
            {
                break;
            }
            array_buffer_0[_S1 + i_0] = array_buffer_0[_S1 + i_0] + u32(1);
            j_0 = j_0 + i32(1);
        }
        i_0 = i_0 + u32(1);
    }
    return;
}

