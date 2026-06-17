#version 330

in vec2 fragTexCoord;
uniform vec2 circleCenter;
uniform float radius;
uniform float screenW;
uniform float screenH;
uniform sampler2D texture0;
out vec4 finalColor;

void main() {
    vec2 uv = fragTexCoord;
    vec2 uvPixelCoord = vec2(uv.x * screenW, (1.0 - uv.y) * screenH);
    vec4 color = texture(texture0, uv);

    float dist = distance(uvPixelCoord, circleCenter);
    if (dist > radius) {
        finalColor = vec4(0., 0., 0., 0.);
    } else {
        float falloff = 1 - (dist/radius);
        finalColor = color * falloff * 3.;
    }
}
