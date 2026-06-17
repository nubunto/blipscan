// defined when compiling under emscripten
#ifdef __EMSCRIPTEN__
#include <emscripten.h>

// defined on the Rust side
extern void set_difficulty(int level);

// a JS function we can call from the Rust side
void notify_game_over(int score) {
  EM_ASM({
    window.dispatchEvent(new CustomEvent('gameover', { detail: $0 }));
  }, score);
}

// a C function that we can call from the Browser side
// (which then calls the Rust side)
EMSCRIPTEN_KEEPALIVE
void js_set_difficulty(int level) {
  set_difficulty(level);
}

#else

// native build: no browser to talk to
// define the symbol anyway, so that everything still works

void notify_game_over(int score) {
  (void)score;
}

#endif
