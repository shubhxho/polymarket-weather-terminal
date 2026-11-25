struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    let x = f32((vertex_index & 1u) << 1u);
    let y = f32((vertex_index & 2u));
    
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.tex_coords = vec2<f32>(x, y);
    
    return out;
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct Uniforms {
    time: f32,
    num_markets: u32,
    max_lead_time: f32,
    padding: f32,
}

@group(0) @binding(1)
var<storage, read> market_data: array<MarketData>;

struct MarketData {
    edge: f32,
    temperature_mean: f32,
    temperature_std: f32,
    lead_time: f32,
}

fn heat_color(value: f32) -> vec3<f32> {
    let t = clamp(value * 0.5 + 0.5, 0.0, 1.0);
    
    var r: f32;
    var g: f32;
    var b: f32;
    
    if (t < 0.25) {
        let s = t / 0.25;
        r = 0.0;
        g = s * 0.5;
        b = 0.5 + s * 0.5;
    } else if (t < 0.5) {
        let s = (t - 0.25) / 0.25;
        r = 0.0;
        g = 0.5 + s * 0.5;
        b = 1.0 - s * 0.5;
    } else if (t < 0.75) {
        let s = (t - 0.5) / 0.25;
        r = s;
        g = 1.0;
        b = 0.5 - s * 0.5;
    } else {
        let s = (t - 0.75) / 0.25;
        r = 1.0;
        g = 1.0 - s * 0.5;
        b = 0.0;
    }
    
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    
    let market_idx = u32(uv.x * f32(uniforms.num_markets));
    let lead_idx = uv.y;
    
    if (market_idx >= uniforms.num_markets) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    
    let data = market_data[market_idx];
    
    let wave = sin(uniforms.time * 2.0 + uv.x * 10.0 + uv.y * 5.0) * 0.1;
    
    let density = exp(-pow((lead_idx * uniforms.max_lead_time - data.lead_time) / data.temperature_std, 2.0) * 0.5);
    
    let edge_intensity = data.edge * 2.0;
    
    let combined_value = density * 0.7 + edge_intensity * 0.3 + wave;
    
    let color = heat_color(combined_value);
    
    let edge_overlay = vec3<f32>(
        step(0.0, data.edge) * data.edge,
        0.0,
        step(data.edge, 0.0) * abs(data.edge)
    );
    
    let final_color = mix(color, edge_overlay, 0.3);
    
    return vec4<f32>(final_color, 1.0);
}
