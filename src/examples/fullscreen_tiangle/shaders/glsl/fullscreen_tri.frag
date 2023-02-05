#version 450
#define AA (4./resolution.y)
#define PI 3.141593
#define TAU 6.283185
#define MAX_FLOAT 1e10;

layout(location = 0) in vec2 uv_in;
layout(location = 0) out vec4 res;

layout(binding=0, std140) uniform Camera
{
    uvec4 resolution;
};

float sdBox(in vec2 p, in vec2 b)
{
    vec2 d = abs(p)-b;
    return length(max(d,0.0)) + min(max(d.x,d.y),0.0);
}

float ndot(vec2 a, vec2 b ) { return a.x*b.x - a.y*b.y; }
float sdRhombus( in vec2 p, in vec2 b ) 
{
    p = abs(p);
    float h = clamp( ndot(b-2.0*p,b)/dot(b,b), -1.0, 1.0 );
    float d = length( p-0.5*b*vec2(1.0-h,1.0+h) );
    return d * sign( p.x*b.y + p.y*b.x - b.x*b.y );
}

void main()
{
    vec2 aspect = vec2(resolution.xy)/vec2(resolution.yy);
    vec2 uv = uv_in * aspect;
    vec2 center = aspect * .5;
    vec2 barycentric = uv - center;
    float ang = atan(barycentric.x, -barycentric.y);
    float ang_norm = ang/TAU + .5;
    vec2 polar = vec2(ang_norm, length(barycentric));
    vec2 uv_mod_5;
    
    float mask = MAX_FLOAT;
    
    { //outer gear
        float outer_gear = .45 + pow(cos(polar.x * PI * 25.), 4.) * .05 - polar.y;
        outer_gear = min(outer_gear, polar.y - .375);
        mask = min(mask, outer_gear);
    }
    
    { //screw holes
        float mod_ang = mod(ang, TAU * .2) - TAU * .1;
        uv_mod_5 = vec2(polar.y * sin(mod_ang), polar.y * cos(mod_ang));
        float screw_holes = -sdRhombus(uv_mod_5 - vec2(0., .5), vec2(.15, .15)) + .035;
        screw_holes = min(screw_holes, .45 - polar.y);
        mask = max(mask, screw_holes);
    }
    
    { // R
        float r = -sdBox(barycentric + vec2(.25, .16), vec2(.25, .05)) + .015;
        float vb = -sdBox(barycentric + vec2(.15, 0.), vec2(.08, .2)) + .015;
        float tb = -sdBox(barycentric + vec2(.2, -.2), vec2(.35, .05)) + .015;
        float rr = -distance(barycentric, vec2(.134, .1355)) + .13;
        
        r = max(r, vb);
        r = max(r, tb);
        r = max(r, rr);
        r = min(r, .45 - polar.y);
        mask = max(mask, r);
    }
    
    { //tail
        float tail = barycentric.y + .221 - (1. - pow(smoothstep(.0, .1, barycentric.x), 2.)) * .19;
        tail = min(tail, -barycentric.y - .175 + (1. - pow(smoothstep(0.1, .25, barycentric.x), 2.)) * .2);
        
        tail = max(tail, -sdBox(barycentric - vec2(.42, -.1), vec2(.1, .075)) + .02);
        tail = max(tail, -sdBox(barycentric - vec2(.35, -.15), vec2(.2, .05)) + .02);
        
        tail = min(tail, .45 - polar.y);
        tail = min(tail, barycentric.x + .1);
        tail = min(tail, distance(barycentric, vec2(.251, -.07)) - .05);
    
        mask = max(mask, tail);
    }
    mask = max(mask, -sdBox(barycentric - vec2(.01, .1), vec2(.135, .15)));
    mask = min(mask, distance(uv_mod_5, vec2(0., .385)) - .03);
    mask = min(mask, max(-barycentric.x + .05, distance(barycentric, vec2(0.1, .11)) - .03));
    mask = min(mask, sdBox(barycentric - vec2(0.02, .11), vec2(.075, .03)));
    
    res = vec4(smoothstep(0., AA, mask));
}