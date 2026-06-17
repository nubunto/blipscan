#version 100

precision mediump float;

varying vec2 fragTexCoord;
uniform sampler2D texture0;

void main() {
    vec2 uv = fragTexCoord;
    // example: CRT scanlines
    vec4 color = texture2D(texture0, uv);
    color.rgb *= 0.8 + 0.2 * sin(uv.y * 600.0 * 3.14159);
    gl_FragColor = color;
}
