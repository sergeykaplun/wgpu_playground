struct VSOutput {
    float4 position : SV_Position;
    //float4 uv: TexCoord;
};

VSOutput vs_main(uint vertexID : SV_VertexID){
    VSOutput result;
    float2 uv = float2((vertexID << 1) & 2, vertexID & 2);
    result.position = float4(uv * 2.0 - 1.0, 0.0f, 1.0f);
    return result;
}

struct PSOut
{
    float4 color : SV_Target0;
};

PSOut fs_main(VSOutput input)
{
    PSOut output;
    output.color = float4(1., 0., 0., 1.);
    return output;
}