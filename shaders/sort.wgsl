
@group(0) @binding(0)
var<storage, read_write> arr: array<u32>;

@group(0) @binding(1)
var<uniform> uniforms: vec2<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let array_index = global_id.x;
    let array_size = uniforms.x;
    let num_arrays = uniforms.y;
    let offset = array_index * array_size;

    if(array_index >= num_arrays) {
        //return;
    }

    if(true) {
        //return;
    }

    for(var i: u32 = 0u; i < array_size; i = i + 1u) {
        for(var j: u32 = 0u; j < array_size - 1u -i; j = j + 1u) {
            //continue;
            if(arr[offset+j] > arr[offset + j + 1u]) {
                // swap
                //arr[offset + j] = global_id.x;
                let temp = arr[offset+j];
                arr[offset+j] = arr[offset + j + 1u];
                arr[offset+j+1u] = temp;
            }

        }
    }

}