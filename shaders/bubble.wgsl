struct ArrayInfo_std140_0
{
    @align(16) array_size_0 : u32,
    @align(4) num_arrays_0 : u32,
};

@binding(1) @group(0) var<uniform> array_info_0 : ArrayInfo_std140_0;
@binding(0) @group(0) var<storage, read_write> array_buffer_0 : array<u32>;

var<workgroup> local_arrays_0 : array<array<u32, i32(32)>, i32(256)>;

@compute
@workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) threadId_0 : vec3<u32>)
{
    var array_index_0 : u32 = threadId_0.x;
    var thread_id_0 : u32 = array_index_0 % u32(256);
    if(array_index_0 >= (array_info_0.num_arrays_0))
    {
        return;
    }
    var _S1 : u32 = array_index_0 * array_info_0.array_size_0;
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
        local_arrays_0[thread_id_0][i_0] = array_buffer_0[_S1 + i_0];
        i_0 = i_0 + u32(1);
    }
    var arr_0 : array<u32, i32(32)> = local_arrays_0[thread_id_0];
    i_0 = u32(0);
    for(;;)
    {
        if(i_0 < (array_info_0.array_size_0))
        {
        }
        else
        {
            break;
        }
        var j_0 : u32 = u32(0);
        for(;;)
        {
            if(j_0 < (array_info_0.array_size_0 - u32(1) - i_0))
            {
            }
            else
            {
                break;
            }
            var _S2 : u32 = j_0 + u32(1);
            if((arr_0[j_0]) > (arr_0[_S2]))
            {
                var _S3 : u32 = arr_0[j_0];
                arr_0[j_0] = arr_0[_S2];
                arr_0[_S2] = _S3;
            }
            j_0 = _S2;
        }
        i_0 = i_0 + u32(1);
    }
    workgroupBarrier();
    i_0 = u32(0);
    for(;;)
    {
        if(i_0 < (array_info_0.array_size_0))
        {
        }
        else
        {
            break;
        }
        array_buffer_0[_S1 + i_0] = arr_0[i_0];
        i_0 = i_0 + u32(1);
    }
    return;
}

