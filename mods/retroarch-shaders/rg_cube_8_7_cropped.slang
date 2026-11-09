#version 450

/*
   RG Cube 8:7 Cropped, RetroArch's copy of GammaOS Next Improved's 8:7 Cropped shader.
   License: Public domain

   The core's frame is stretched to a projection as tall as the output's shorter side and 10/9 as
   wide (800 x 720 on the 720 x 720 panel, 1200 x 1080 on a 1920 x 1080 output, whatever
   resolution RetroArch renders at), centred and cropped to the output, with black around it. Every frame is projected, whatever its size.

   Set Aspect Ratio to Full so the viewport is the whole screen.

   Sampling is sharp bilinear: each texel is held flat and blended with its neighbour only
   across the output pixel on their shared edge.
*/

layout(push_constant) uniform Push
{
  vec4 OutputSize;
  vec4 OriginalSize;
  vec4 SourceSize;
} params;

layout(std140, set = 0, binding = 0) uniform UBO
{
   mat4 MVP;
} global;

#pragma stage vertex
layout(location = 0) in vec4 Position;
layout(location = 1) in vec2 TexCoord;
layout(location = 0) out vec2 vTexCoord;

void main()
{
   gl_Position = global.MVP * Position;
   vTexCoord = TexCoord;
}

#pragma stage fragment
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
layout(set = 0, binding = 2) uniform sampler2D Source;

void main()
{
   vec2 src = params.SourceSize.xy;
   vec2 outSize = params.OutputSize.xy;
   float side = min(outSize.x, outSize.y);
   vec2 proj = vec2(side * 10.0 / 9.0, side);

   vec2 t = (vTexCoord * outSize - (outSize - proj) * 0.5) * src / proj;
   if (any(lessThan(t, vec2(0.0))) || any(greaterThan(t, src)))
   {
      FragColor = vec4(0.0, 0.0, 0.0, 1.0);
      return;
   }

   vec2 scale = max(proj / src, vec2(1.0));
   vec2 c = t - 0.5;
   vec2 i = floor(c);
   vec2 f = clamp((c - i - 0.5) * scale + 0.5, 0.0, 1.0);
   vec2 uv = clamp(i + f + 0.5, vec2(0.5), src - 0.5) * params.SourceSize.zw;
   FragColor = vec4(texture(Source, uv).rgb, 1.0);
}
