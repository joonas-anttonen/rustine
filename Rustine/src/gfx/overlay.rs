let simple_shader_str = r#"
struct PerCommand
{
	float2 Scale;
    bool isSdf;
	float sdfRange;
};

[[vk::push_constant]] PerCommand command;

[[vk::binding(0, 0)]] Texture2D commandTexture;
[[vk::binding(1, 0)]] SamplerState commandSampler;

struct vertex_input
{
	float2 Position : POSITION0;
	float2 UV : TEXCOORD0;
	uint Color : COLOR0;
};

struct fragment_input
{
	float4 Position : SV_POSITION;
	float2 UV : TEXCOORD0;
	float4 Color : COLOR0;
};

float4 UnpackColor(uint packed)
{
    float r = (float)(packed & 0xFF) / 255.0f;
    float g = (float)((packed >> 8) & 0xFF) / 255.0f;
    float b = (float)((packed >> 16) & 0xFF) / 255.0f;
    float a = (float)((packed >> 24) & 0xFF) / 255.0f;
    return float4(r, g, b, a);
}

[shader("vertex")]
fragment_input vertex(vertex_input input, in uint vertexIndex : SV_VertexID)
{
    fragment_input output = (fragment_input)0;
    output.Position = float4(input.Position * command.Scale + float2(-1, -1), 0.0, 1.0);
	output.UV = input.UV;
	output.Color = UnpackColor(input.Color);
	return output;
}

[shader("pixel")]
float4 fragment(fragment_input input) : SV_TARGET
{ 
	float4 geometryColor = input.Color;
	float4 textureColor = commandTexture.Sample(commandSampler, input.UV);

    float alpha = textureColor.a;

	if (command.isSdf)
	{ 
		float sdf = textureColor.a - 0.5;
		alpha = smoothstep(-command.sdfRange, +command.sdfRange, sdf);
	}

	return float4(geometryColor.rgb * textureColor.rgb, alpha * geometryColor.a);
}"#;