const R_SMOOTH : f32 = 2.0;

[[block]]
struct Positions {
    data : [[stride(8)]] array< vec2<f32> >;
};

[[block]]
struct Velocities {
    data : [[stride(8)]] array< vec2<f32> >;
};

[[block]]
struct Types {
    data : [[stride(4)]] array<u32>;
};

[[block]]
struct CacheRadius {
    data : [[stride(4)]] array<f32>;
};

[[block]]
struct CacheAttraction {
    data : [[stride(4)]] array<f32>;
};

[[block]]
struct Globals {
    num_points : u32;
    num_types : u32;
    friction : f32;
    wrapping: u32;
    dist: f32;
};

[[group(0), binding(0)]] var<storage> positions : [[access(read_write)]] Positions;
[[group(0), binding(1)]] var<storage> positions_old : [[access(read)]] Positions;
[[group(0), binding(2)]] var<storage> velocities : [[access(read_write)]] Velocities;
[[group(0), binding(3)]] var<uniform> types : Types;
[[group(0), binding(4)]] var<uniform> cache_max_r : CacheRadius;
[[group(0), binding(5)]] var<uniform> cache_min_r : CacheRadius;
[[group(0), binding(6)]] var<uniform> cache_attraction : CacheAttraction;
[[group(0), binding(7)]] var<uniform> globals : Globals;

[[builtin(global_invocation_id)]] var<in> global_invocation_id : vec3<u32>;

fn tovec(float : f32) -> vec2<f32> {
    return vec2<f32>(float, float);
}

[[stage(compute), workgroup_size(256)]]
fn main() -> void {
    var i : u32 = global_invocation_id.x;
    if (i >= globals.num_points) {
        return;
    }
    var p : vec2<f32> = positions_old.data[i];
    var p_type : u32 = types.data[i];

    var j : u32 = 0u;
    loop {
        var q : vec2<f32> = positions_old.data[j];
        var q_type : u32 = types.data[j];
        var pair_idx : u32 = (p_type * globals.num_types) + q_type;
        var delta : vec2<f32> = q - p;

        if (globals.wrapping != 0u) {
            if (delta.x > globals.dist) {
                delta.x = delta.x - globals.dist * 2.0;
            } else {
                if (delta.x < -globals.dist) {
                    delta.x = delta.x + globals.dist * 2.0;
                }
            }

            if (delta.y > globals.dist) {
                delta.y = delta.y - globals.dist * 2.0;
            } else {
                if (delta.y < -globals.dist) {
                    delta.y = delta.y + globals.dist * 2.0;
                }
            }
        }

        var r2 : f32 = delta.x * delta.x + delta.y * delta.y;
        var max_r : f32 = cache_max_r.data[pair_idx];

        if (r2 > max_r * max_r || r2 < 0.01) {
            continue;
        }

        var min_r : f32 = cache_min_r.data[pair_idx];
        var attraction : f32 = cache_attraction.data[pair_idx];

        var r : f32 = sqrt(r2);
        delta = delta / tovec(r);

        var f : f32;
        if (r > min_r) {
            var numer : f32 = 2.0 * abs(r - 0.5 * (max_r + min_r));
            var denom : f32 = max_r - min_r;
            f = attraction * (1.0 - numer / denom);
        } else {
            f = R_SMOOTH * min_r * (1.0 / (min_r + R_SMOOTH) - 1.0 / (r + R_SMOOTH));
        }

        velocities.data[i] = velocities.data[i] + delta * tovec(f);

        continuing {
            j = j + 1u;
            if (j >= globals.num_points) {
                break;
            }
        }
    }

    positions.data[i] = positions.data[i] + velocities.data[i];
    velocities.data[i] = velocities.data[i] * tovec(1.0 - globals.friction);

    if (globals.wrapping) {
        if (positions.data[i].x < -globals.dist) {
            positions.data[i].x = positions.data[i].x + globals.dist * 2.0;
        } else {
            if (positions.data[i].x >= globals.dist) {
                positions.data[i].x = positions.data[i].x - globals.dist * 2.0;
            }
        }

        if (positions.data[i].y < -globals.dist) {
            positions.data[i].y = positions.data[i].y + globals.dist * 2.0;
        } else {
            if (positions.data[i].y >= globals.dist) {
                positions.data[i].y = positions.data[i].y - globals.dist * 2.0;
            }
        }
    } else {
        if (globals.dist != 0.0) {
            if (positions.data[i].x < -globals.dist) {
                velocities.data[i].x = -velocities.data[i].x;
                positions.data[i].x = -globals.dist;
            } else {
                if (positions.data[i].x >= globals.dist) {
                    velocities.data[i].x = -velocities.data[i].x;
                    positions.data[i].x = globals.dist;
                }
            }

            if (positions.data[i].y < -globals.dist) {
                velocities.data[i].y = -velocities.data[i].y;
                positions.data[i].y = -globals.dist;
            } else {
                if (positions.data[i].y >= globals.dist) {
                    velocities.data[i].y = -velocities.data[i].y;
                    positions.data[i].y = globals.dist;
                }
            }
        }
    }
}
