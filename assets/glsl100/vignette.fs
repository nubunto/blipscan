#version 100

precision mediump float;

varying vec2 fragTexCoord;
uniform sampler2D texture0;

void main() {
    vec4 color = texture2D(texture0, fragTexCoord);

    // distance from center (0 at center, ~0.7 at corners)
    vec2 uv = fragTexCoord - 0.5;
    float vignette = 1.0 - dot(uv, uv) * 2.0;

    gl_FragColor = vec4(color.rgb * vignette, color.a);
}
