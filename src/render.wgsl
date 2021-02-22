[[builtin(vertex_index)]]
var<in> in_vertex_index : u32;
[[builtin(instance_index)]]
var<in> in_instance_index : u32;
[[builtin(position)]]
var<out> out_pos: vec4<f32>;
[[location(0)]]
var<in> in_pos: vec2<f32>;
[[location(1)]]
var<in> in_point_pos: vec2<f32>;
[[location(0)]]
var<out> out_color: vec3<f32>;

[[block]]
struct Types {
    data : [[stride(4)]] array<u32>;
};

[[block]]
struct Colors {
    data: [[stride(12)]] array< vec3<f32> >;
};

[[block]]
struct Globals {
    num_points : u32;
    num_types: u32;
    friction : f32;
};

[[block]]
struct RenderGlobals {
    x : f32;
    y : f32;
    width : u32;
    height : u32;
    zoom : f32;
};

[[group(0), binding(0)]] var<uniform> globals : Globals;
[[group(0), binding(1)]] var<uniform> render_globals : RenderGlobals;
[[group(0), binding(2)]] var<storage> types : [[access(read)]] Types;
[[group(0), binding(3)]] var<storage> colors : [[access(read)]] Colors;

[[stage(vertex)]]
fn main() {
    var width : f32 = f32(render_globals.width);
    var height : f32 = f32(render_globals.height);
    var camera_pos : vec2<f32> = vec2<f32>(render_globals.x, render_globals.y);
    var size : vec2<f32> = vec2<f32>(width, height);
    var smallest_side : f32 = min(width, height);
    var aspect_ratio : vec2<f32> = size / vec2<f32>(smallest_side, smallest_side);
    var pos : vec2<f32> = (in_point_pos + in_pos - camera_pos) / aspect_ratio * vec2<f32>(render_globals.zoom, render_globals.zoom);
    out_pos = vec4<f32>(pos, 0.0, 1.0);
    out_color = colors.data[ types.data[in_instance_index] ];
}

[[builtin(frag_coord)]] var<in> frag_coord : vec4<f32>;

[[location(0)]]
var<out> out_color: vec4<f32>;
[[location(0)]]
var<in> in_color: vec3<f32>;

[[stage(fragment)]]
fn main() {
    //var i : u32 = 0u;
    //var c : u32 = 0u;
    //var color : vec3<f32>;
    //loop {
    //    if ((distance(pos, positions.data[i])) < 5.0) {
    //        c = 1u;
    //        color = colors.data[ types.data[i] ];
    //        break;
    //    }
    //    continuing {
    //        i = i + 1;
    //        if (i >= globals.num_points) {
    //            break;
    //        }
    //    }
    //}
    //if (c == 1u) {
    //    out_color = vec4<f32>(color, 1.0);
    //} else {
    out_color = vec4<f32>(in_color, 1.0);
    //}
}