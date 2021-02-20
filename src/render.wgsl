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

[[block]]
struct RenderGlobals {
    x : f32;
    y : f32;
    width : u32;
    height : u32;
    zoom : f32;
};

[[group(0), binding(0)]] var<storage> positions : [[access(read)]] Positions;
[[group(0), binding(1)]] var<uniform> globals : Globals;
[[group(0), binding(2)]] var<uniform> types : Types;
[[group(0), binding(3)]] var<uniform> colors : Colors;
[[group(0), binding(4)]] var<uniform> render_globals : RenderGlobals;

[[builtin(frag_coord)]] var<in> frag_coord : vec4<f32>;

[[location(0)]]
var<out> out_color: vec4<f32>;

[[stage(fragment)]]
fn main() {
    var width : f32 = f32(render_globals.width);
    var height : f32 = f32(render_globals.height);
    var camera_pos : vec2<f32> = vec2<f32>(render_globals.x, render_globals.y);
    var size : vec2<f32> = vec2<f32>(width, height);
    var smallest_side : f32 = min(width, height);
    var square_size : vec2<f32> = vec2<f32>(smallest_side, smallest_side);
    var normalized : vec2<f32> = (frag_coord.xy - vec2<f32>(width / 2.0, height / 2.0)) / square_size;
    var pos : vec2<f32> = normalized / vec2<f32>(render_globals.zoom, render_globals.zoom) + camera_pos;
    var i : u32 = 0u;
    var c : u32 = 0u;
    var color : vec3<f32>;
    loop {
        if ((distance(pos, positions.data[i])) < 5.0) {
            c = 1u;
            color = colors.data[ types.data[i] ];
            break;
        }
        continuing {
            i = i + 1;
            if (i >= globals.num_points) {
                break;
            }
        }
    }
    if (c == 1u) {
        out_color = vec4<f32>(color, 1.0);
    } else {
        out_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}