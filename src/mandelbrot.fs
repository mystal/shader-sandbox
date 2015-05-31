uniform vec2 screenSize;

bool mandelbrotConverges(vec2 z) {
  return length(z) < 2.0;
}

vec2 mandelbrotIter(vec2 z, vec2 c) {
  return vec2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
}

bool mandelbrot(vec2 c) {
  //Test if the point c is inside the mandelbrot set after 100 iterations
  vec2 z = vec2(0.0);
  vec2 zrun = z;
  for (int i = 0; i < 100; i++) {
    zrun = mandelbrotIter(zrun, c);
  }

  return mandelbrotConverges(zrun);
}

void main() {
    // TODO: gl_FragCoord is in pixel coordinates. Map to relative coordinates.
    vec2 coord = gl_FragCoord.xy / screenSize;
    if (mandelbrot(coord)) {
        gl_FragColor = vec4(1.0);
    } else {
        gl_FragColor = vec4(0.0);
    }
}
