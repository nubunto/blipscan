#version 100

precision mediump float;

varying vec2 fragTexCoord;
uniform vec2 circleCenter;
uniform float radius;
uniform float screenW;
uniform float screenH;
uniform sampler2D texture0;

void main() {
    vec2 uv = fragTexCoord;
    vec2 uvPixelCoord = vec2(uv.x * screenW, (1.0 - uv.y) * screenH);
    vec4 color = texture2D(texture0, uv);

    float dist = distance(uvPixelCoord, circleCenter);
    if (dist > radius) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 0.0);
    } else {
        float falloff = 1.0 - (dist / radius);
        gl_FragColor = color * falloff * 3.0;
    }
}
