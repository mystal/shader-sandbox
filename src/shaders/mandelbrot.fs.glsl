uniform vec3 iResolution;
uniform int iterations;

bool mandelbrotConverges(vec2 z) {
  return length(z) < 2.0;
}

vec2 mandelbrotIter(vec2 z, vec2 c) {
  return vec2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
}

bool mandelbrot(vec2 c) {
  // Test if the point c is in the mandelbrot set after iterations.
  vec2 z = vec2(0.0);
  for (int i = 0; i < iterations; i++) {
    z = mandelbrotIter(z, c);
  }

  return mandelbrotConverges(z);
}

void main() {
    vec2 c = 2.0 * (gl_FragCoord.xy / iResolution.xy) - vec2(1.0, 1.0);
    c.x *= iResolution.x / iResolution.y;
    c.x -= 0.5;
    if (mandelbrot(c)) {
        gl_FragColor = vec4(1.0);
    } else {
        gl_FragColor = vec4(0.0);
    }
}
