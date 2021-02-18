[[builtin(vertex_index)]]
var<in> in_vertex_index: u32;
[[builtin(position)]]
var<out> out_pos: vec4<f32>;

var verts : array<vec2<f32>, 6u> = array<vec2<f32>, 6u>(
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(1.0, 1.0)
);

[[stage(vertex)]]
fn main() {
    out_pos = vec4<f32>(verts[in_vertex_index], 0.0, 1.0);
}

[[block]]
struct Positions {
    data : [[stride(8)]] array< vec2<f32> >;
};

[[block]]
struct Globals {
    num_points : u32;
    num_types: u32;
    friction : f32;
};

[[block]]
struct Types {
    data : [[stride(4)]] array<u32>;
};

[[block]]
struct Colors {
    data: [[stride(12)]] array< vec3<f32> >;
};

[[group(0), binding(0)]] var<storage> positions : [[access(read)]] Positions;
[[group(0), binding(1)]] var<uniform> globals : Globals;
[[group(0), binding(2)]] var<uniform> types : Types;
[[group(0), binding(3)]] var<uniform> colors : Colors;

[[builtin(frag_coord)]] var<in> frag_coord : vec4<f32>;

[[location(0)]]
var<out> out_color: vec4<f32>;

[[stage(fragment)]]
fn main() {
    var pos : vec2<f32> = (frag_coord.xy / vec2<f32>(800.0, 600.0) - vec2<f32>(0.5, 0.5)) * vec2<f32>(2.0, 2.0) * vec2<f32>(500.0, 500.0);
    var i : u32 = 0u;
    var c : f32 = 10000.0;
    var color : vec3<f32>;
    loop {
        var c_new : f32 = distance(pos, positions.data[i]);
        if (c_new < c) {
            c = c_new;
            color = colors.data[ types.data[i] ];
        }
        continuing {
            i = i + 1;
            if (i >= globals.num_points) {
                break;
            }
        }
    }
    c = clamp(c, 0.0, 1.0);
    c = 1.0 - c;
    out_color = vec4<f32>(color * vec3<f32>(c, c, c), 1.0);
}