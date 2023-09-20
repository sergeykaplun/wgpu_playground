struct CameraParams {
    projection :                      mat4x4<f32>,
    model :                           mat4x4<f32>,
    view :                            mat4x4<f32>,
    position :                        vec3<f32>,
};
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>
};
struct Ray {
    origin :                          vec3<f32>,
    direction :                       vec3<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraParams;
@group(1) @binding(0) var t_volume: texture_3d<f32>;
@group(1) @binding(1) var s_volume: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = in.position;
    out.clip_pos = camera.projection * camera.view * vec4<f32>(out.world_pos, 1.0);
    out.clip_pos = out.clip_pos.xyww;
    out.normal = in.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let camRay =  Ray(in.world_pos, normalize(in.world_pos - camera.position));
    return raymarch(camRay);// + vec4<f32>(0.2, 0.0, 0.0, 0.2);
}

fn raymarch(ray: Ray) -> vec4<f32> {
    let steps_count = 200;
    let diagon = 2. * sqrt(3.);
    let max_dist = box_out_dist(ray, vec3<f32>(-1.0, -1.0, -1.0), vec3<f32>(1.0, 1.0, 1.0));
    let step = diagon / f32(steps_count);
    //let step = max_dist / f32(steps_count);
    var t = 0.0;
    var res = vec4<f32>(0.0);
    for (var i = 0; i < steps_count; i+=1) {
        let p = ray.origin + ray.direction * t;
        res += textureSample(t_volume, s_volume, p * 0.5 + 0.5) * 1.;
        //res += step(distance(p, vec3<f32>(0.0)), 0.5) * .025;
        t += step;
        if(t > max_dist) {
            break;
        }
        if(res.a >= 1.0) {
            break;
        }
    }
    return res;
}

fn box_out_dist(ray: Ray, box_min: vec3<f32>, box_max: vec3<f32>) -> f32 {
    let inverse_dir = 1.0 / ray.direction;
    let tbot = inverse_dir * (box_min - ray.origin);
    let ttop = inverse_dir * (box_max - ray.origin);
    let tmax = max(ttop, tbot);
    let traverse = min(tmax.xx, tmax.yz);
    return min(traverse.x, traverse.y);
}