#version 100
precision mediump float;

varying vec4 fragColor;
varying vec2 fragTexCoord;

uniform sampler2D texture0;

vec2 CRTCurveUV(vec2 uv)
{
    uv = uv * 2.0 - 1.0;
    vec2 offset = abs(uv.yx) / vec2(6.0, 4.0);
    uv = uv + uv * offset * offset;
    uv = uv * 0.5 + 0.5;
    return uv;
}

void DrawVignette(inout vec3 color, vec2 uv)
{
    float vignette = uv.x * uv.y * (1.0 - uv.x) * (1.0 - uv.y);
    vignette = clamp(pow(16.0 * vignette, 0.3), 0.0, 1.0);
    color *= vignette;
}

void DrawScanline(inout vec3 color, vec2 uv)
{
    float iTime = 0.1;
    float scanline = clamp(
        0.95 + 0.05 * cos(3.14159 * (uv.y + 0.008 * iTime) * 240.0),
        0.0,
        1.0
    );

    float grille = 0.85 + 0.15 * clamp(
        1.5 * cos(3.14159 * uv.x * 640.0),
        0.0,
        1.0
    );

    color *= scanline * grille * 1.2;
}

void main() {
    vec2 uv = fragTexCoord;
    vec2 crtUV = CRTCurveUV(uv);

    if (crtUV.x < 0.0 || crtUV.x > 1.0 || crtUV.y < 0.0 || crtUV.y > 1.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    vec4 tex = texture2D(texture0, crtUV);
    vec3 res = tex.rgb * fragColor.rgb;

    DrawVignette(res, crtUV);
    DrawScanline(res, uv);

    gl_FragColor = vec4(res, tex.a * fragColor.a);
}
