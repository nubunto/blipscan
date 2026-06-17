use std::collections::VecDeque;

use rand::RngExt;
use raylib::prelude::*;

// TODO: will use it for a leaderboard in the future
// will probably show some UI on HTML
unsafe extern "C" {
    fn notify_game_over(score: i32);
}

// TODO: unecessary now, shows how to call into rust from the web
#[unsafe(no_mangle)]
pub extern "C" fn set_difficulty(level: i32) {
    println!("level from html: {level}");
}

// general consts
const SCREEN_DIMENSIONS: (i32, i32) = (640, 480);
const TITLE: &str = "BLIPSCAN";
const GAME_OVER: &str = "GAME OVER";
const FONT_SIZE: i32 = 25;
const INITIAL_LIVES: i32 = 3;

// blip related consts
const BLIP_RADIUS: f32 = 5.;
const INITIAL_MAX_BLIPS: i32 = 3;
const BLIP_SPAWN_INTERVAL: f32 = 8.;
const NEW_BLIP_PULSE_INTERVAL: f32 = 0.13;

// ripple consts
// could also be properties on the ripple, if we wanted to make them more interesting
const RIPPLE_DURATION: f32 = 0.6;
const RIPPLE_MAX_RADIUS: f32 = 45.;

trait RaylibDrawExt: RaylibDraw {
    fn draw_text_centered(&mut self, text: &str, font_size: i32, text_width: Vector2, color: Color);
}

impl<T: RaylibDraw> RaylibDrawExt for T {
    fn draw_text_centered(
        &mut self,
        text: &str,
        font_size: i32,
        text_width: Vector2,
        color: Color,
    ) {
        let (sw, sh) = (SCREEN_DIMENSIONS.0 as f32, SCREEN_DIMENSIONS.1 as f32);
        let pos = {
            let x = (sw / 2.) - (text_width.x / 2.);
            let y = (sh / 2.) - (font_size as f32 / 2.);
            Vector2::new(x, y)
        };
        self.draw_text(text, pos.x as i32, pos.y as i32, font_size, color);
    }
}

// A ripple, a set of drawing rings that fades with time
// used for feedback whenever a blip dies of "old age"
struct Ripple {
    position: Vector2,
    age: f32,
}

impl Ripple {
    fn new(position: Vector2) -> Self {
        Self { position, age: 0. }
    }

    fn finished(&self) -> bool {
        self.age >= RIPPLE_DURATION
    }

    fn update(&mut self, dt: f32) {
        self.age += dt;
    }

    fn draw<T: RaylibDraw>(&self, d: &mut T) {
        let t = (self.age / RIPPLE_DURATION).clamp(0., 1.);
        let radius = RIPPLE_MAX_RADIUS * t.sqrt();
        let alpha = (1. - t) * 255.;
        let thickness = 1. + 4. * (1. - t);
        let color = Color::new(85, 26, 139, alpha as u8);
        d.draw_ring(
            self.position,
            (radius - thickness).max(0.),
            radius,
            0.,
            360.,
            32,
            color,
        );
    }
}

// Pulse timer. Fires after `interval`, `remaining` times.
// Currently used to drive some sounds whenever a blip is spawned.
struct Pulse {
    interval: f32,
    time: f32,
    remaining: i32,
}

impl Pulse {
    fn new(interval: f32) -> Self {
        Self {
            interval,
            time: interval,
            remaining: 0,
        }
    }

    fn arm(&mut self, count: i32) {
        self.remaining = count;
        self.time = 0.;
    }

    fn tick(&mut self, dt: f32) -> bool {
        if self.remaining <= 0 {
            return false;
        }

        self.time -= dt;
        if self.time <= 0. {
            self.time += self.interval;
            self.remaining -= 1;
            return true;
        }

        false
    }
}

// Normal timer. can be recurring, or one-shot.
struct Timer {
    original_duration: f32,
    time: f32,
    recurring: bool,
    active: bool,
}

impl Timer {
    fn recurring(duration: f32) -> Self {
        Self {
            original_duration: duration,
            time: duration,
            recurring: true,
            active: true,
        }
    }

    fn tick(&mut self, time_advance: f32) -> bool {
        if !self.active {
            return false;
        }

        self.time = (self.time - time_advance).max(0.);
        if self.time <= 0. && self.recurring {
            self.time = self.original_duration;
            return true;
        }

        if self.time <= 0. {
            return true;
        }

        false
    }
}

#[derive(Debug, Clone, Copy)]
enum BlipState {
    Alive,
    Dead,
    Killed,
}

// A blip. The thing you need to click on.
// If it dies of "old age" (i.e. lifetime <= 0.), you lose a life
struct Blip {
    position: Vector2,
    lifetime: f32,
    state: BlipState,
}

impl Blip {
    fn new(position: Vector2, lifetime: f32) -> Self {
        Self {
            position,
            lifetime,
            state: BlipState::Alive,
        }
    }

    fn alive(&self) -> bool {
        matches!(self.state, BlipState::Alive)
    }

    fn draw<T: RaylibDraw>(&self, d: &mut T) {
        if !self.alive() {
            return;
        }
        d.draw_circle_v(self.position, BLIP_RADIUS, Color::REBECCAPURPLE);
    }

    fn update(&mut self, dt: f32) {
        if !self.alive() {
            return;
        }
        self.lifetime = (self.lifetime - dt).max(0.);
        if self.lifetime <= 0. {
            self.state = BlipState::Dead;
        }
    }
}

enum GamePhase {
    Start,
    Play,
    GameOver,
}

// The main app state
// Contains pretty much everything that drives the game forward:
// - spawns blips
// - handles sounds
// - handles ripples
// - advances times
// Pretty much the main game loop that connects all the entities.
struct AppState<'a> {
    rng: rand::rngs::ThreadRng,
    max_blips_per_turn: i32,
    blip_timer: Timer,
    new_blip_pulse: Pulse,
    new_blip_sound: Sound<'a>,
    dead_blip_sound: Sound<'a>,
    killed_blip_sound: Vec<Sound<'a>>,
    killed_blip_sound_queue: VecDeque<usize>,
    killed_blip_sound_playing: Option<usize>,
    ripples: Vec<Ripple>,
    blips: Vec<Blip>,

    score: i32,
    lives: i32,
    phase: GamePhase,

    radar: Radar,
}

impl<'a> AppState<'a> {
    pub fn new(
        trng: rand::rngs::ThreadRng,
        radar: Radar,
        new_blip_sound: Sound<'a>,
        dead_blip_sound: Sound<'a>,
        killed_blip_sound: Vec<Sound<'a>>,
    ) -> Self {
        let mut s = Self {
            rng: trng,
            new_blip_sound,
            dead_blip_sound,
            killed_blip_sound,
            radar,
            // Gameplay fields below are immediately repopulated by reset();
            // seeded with valid initial values.
            max_blips_per_turn: INITIAL_MAX_BLIPS,
            blip_timer: Timer::recurring(BLIP_SPAWN_INTERVAL),
            new_blip_pulse: Pulse::new(NEW_BLIP_PULSE_INTERVAL),
            killed_blip_sound_queue: VecDeque::new(),
            killed_blip_sound_playing: None,
            ripples: Vec::new(),
            blips: Vec::new(),
            score: 0,
            lives: INITIAL_LIVES,
            phase: GamePhase::Start,
        };

        s.reset();
        s.phase = GamePhase::Start;
        s
    }

    fn reset(&mut self) {
        self.max_blips_per_turn = INITIAL_MAX_BLIPS;
        self.blip_timer = Timer::recurring(BLIP_SPAWN_INTERVAL);
        self.new_blip_pulse = Pulse::new(NEW_BLIP_PULSE_INTERVAL);
        self.killed_blip_sound_queue.clear();
        self.killed_blip_sound_playing = None;
        self.ripples.clear();
        self.blips.clear();
        self.score = 0;
        self.lives = INITIAL_LIVES;
        self.phase = GamePhase::Play;
        self.push_random_blips();
    }

    fn push_random_blips(&mut self) {
        let blips_per_turn = self.rng.random_range(1..self.max_blips_per_turn);
        for _ in 0..blips_per_turn {
            let random_pos = (
                self.rng.random_range(20. ..450.),
                self.rng.random_range(25. ..450.),
            );
            self.blips.push(Blip::new(
                random_pos.into(),
                self.rng.random_range(7. ..13.),
            ));
        }

        self.new_blip_pulse.arm(blips_per_turn);
    }

    pub fn draw<T: RaylibDraw>(&self, d: &mut T, title_text_width: i32, gameover_text_width: i32) {
        match self.phase {
            GamePhase::Start => self.draw_start(d, title_text_width),
            GamePhase::Play => self.draw_play(d),
            GamePhase::GameOver => self.draw_game_over(d, gameover_text_width),
        }
    }

    fn draw_start<T: RaylibDraw>(&self, d: &mut T, text_width: i32) {
        d.draw_text_centered(
            "BLIPSCAN",
            20,
            Vector2::new(text_width as f32, text_width as f32),
            Color::MEDIUMVIOLETRED,
        );
        d.draw_text(
            "Press ENTER to start",
            (SCREEN_DIMENSIONS.0 / 2) - 61,
            (SCREEN_DIMENSIONS.1 / 2) + 10,
            20,
            Color::BLACK,
        );
    }

    fn draw_game_over<T: RaylibDraw>(&self, d: &mut T, text_width: i32) {
        let tw = Vector2::new(text_width as f32, text_width as f32);
        d.draw_text_centered("GAME OVER", 30, tw, Color::MEDIUMVIOLETRED);
        d.draw_text(
            "Press R to restart",
            (SCREEN_DIMENSIONS.0 / 2) - 61,
            (SCREEN_DIMENSIONS.1 / 2) + 10,
            20,
            Color::BLACK,
        );
    }

    fn draw_play<T: RaylibDraw>(&self, d: &mut T) {
        self.radar.draw(d);
        for rip in &self.ripples {
            rip.draw(d);
        }

        d.draw_text(&format!("Lives: {}", self.lives), 40, 45, 20, Color::BLACK);
        d.draw_text(&format!("Kills: {}", self.score), 40, 65, 20, Color::BLACK);
        d.draw_text(
            &format!("{}", self.blips.len()),
            self.radar.center.x as i32,
            self.radar.center.y as i32 - 3,
            25,
            Color::BLACK,
        );
    }

    pub fn update(&mut self, rl: &mut RaylibHandle, dt: f32) {
        match self.phase {
            GamePhase::Start => {
                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    self.phase = GamePhase::Play;
                }
                return;
            }
            GamePhase::GameOver => {
                if rl.is_key_pressed(KeyboardKey::KEY_R) {
                    self.reset();
                }
                return;
            }
            GamePhase::Play => {}
        }

        if self.blip_timer.tick(dt) {
            self.push_random_blips();
            self.max_blips_per_turn += 1;
        }

        if self.new_blip_pulse.tick(dt) {
            self.new_blip_sound.play();
        }

        // simulates "mouse click and release" sound
        // this is all so that we play one sound after another
        // lmao... gamedev is fun
        let busy = self
            .killed_blip_sound_playing
            .is_some_and(|i| self.killed_blip_sound[i].is_playing());

        if !busy && let Some(i) = self.killed_blip_sound_queue.pop_front() {
            self.killed_blip_sound[i].play();
            self.killed_blip_sound_playing = Some(i);
        }

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            for blip in &mut self.blips {
                // did we collide with this blip?
                if raylib::check_collision_circles(
                    self.radar.center,
                    self.radar.radius,
                    blip.position,
                    BLIP_RADIUS,
                ) {
                    blip.state = BlipState::Killed;
                    self.score += 1;
                    // play first then second audio
                    // uses indexes, we dont want bad lifetimes here!
                    self.killed_blip_sound_queue.push_back(0);
                    self.killed_blip_sound_queue.push_back(1);
                }
            }
        }

        self.radar.update(rl, dt);

        for blip in &mut self.blips {
            let prev_blip_state = blip.state;
            blip.update(dt);
            let new_blip_state = blip.state;
            // if "was alive" and now "is dead"
            if let (BlipState::Alive, BlipState::Dead) = (prev_blip_state, new_blip_state) {
                self.lives -= 1;
                self.dead_blip_sound.play();
                self.ripples.push(Ripple::new(blip.position));
            }
        }

        for rip in &mut self.ripples {
            rip.update(dt);
        }

        // retain only unfinished ripples/live blips
        self.ripples.retain(|r| !r.finished());
        self.blips.retain(|b| b.alive());

        if self.lives <= 0 {
            self.phase = GamePhase::GameOver;
            unsafe {
                // Im sorry, we need this here. Its ffi with the browser!
                notify_game_over(self.score);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut rl, thread) = raylib::init()
        .size(SCREEN_DIMENSIONS.0, SCREEN_DIMENSIONS.1)
        .build();

    // game_loop::run requires a 'static closure
    // so we need to leak this shit here
    // its fine, OS reclaims it on native and browser reclaims it on tab closed
    let ra = Box::leak(Box::new(RaylibAudio::init_audio_device()?));
    let radar = Radar::new(Vector2::one(), 105.);

    let (screen_width, screen_height) = { (rl.get_screen_width(), rl.get_screen_height()) };

    // the idea: we render the main app to this texture...
    let mut app_render_texture =
        rl.load_render_texture(&thread, screen_width as u32, screen_height as u32)?;
    // we render the blips to this texture, and then we apply the circular_clipping shader HERE...
    let mut blips_render_texture =
        rl.load_render_texture(&thread, screen_width as u32, screen_height as u32)?;
    // then we render the above 2 textures onto this texture, with the 1st post-processing pass...
    let mut final_render_texture =
        rl.load_render_texture(&thread, screen_width as u32, screen_height as u32)?;

    // and THEN we apply the 2nd pass of the post-processing
    let mut pass1_render_texture =
        rl.load_render_texture(&thread, screen_width as u32, screen_height as u32)?;

    // raylib's web build targets OpenGL ES 2.0 (WebGL), which wants GLSL ES
    // 1.00 shaders; the desktop build wants GLSL 330. Keep both and pick the
    // matching folder at compile time.
    // one day we might not have to duplicate shaders...
    #[cfg(target_os = "emscripten")]
    const GLSL_DIR: &str = "assets/glsl100";
    #[cfg(not(target_os = "emscripten"))]
    const GLSL_DIR: &str = "assets/glsl330";

    let mut shader_circular_clipping = rl.load_shader(
        &thread,
        None,
        Some(&format!("{GLSL_DIR}/circular_clipping.fs")),
    )?;
    let mut shader_crt = rl.load_shader(&thread, None, Some(&format!("{GLSL_DIR}/crt.fs")))?;
    let mut shader_vignette =
        rl.load_shader(&thread, None, Some(&format!("{GLSL_DIR}/vignette.fs")))?;

    let mut post_processing_shaders_enabled = true;

    let rng = rand::rngs::ThreadRng::default();

    let circle_center_loc = shader_circular_clipping.get_shader_location("circleCenter");
    let circle_radius_loc = shader_circular_clipping.get_shader_location("radius");
    let screen_width_loc = shader_circular_clipping.get_shader_location("screenW");
    let screen_height_loc = shader_circular_clipping.get_shader_location("screenH");

    game_loop::run(rl, thread, 60, {
        let new_blip_sound = ra.new_sound("assets/switch31.wav")?;
        let dead_blip_sound = ra.new_sound("assets/rollover6.wav")?;
        let killed_blip_sound = vec![
            ra.new_sound("assets/mouseclick1.wav")?,
            ra.new_sound("assets/mouserelease1.wav")?,
        ];
        let mut app_state = AppState::new(
            rng,
            radar,
            new_blip_sound,
            dead_blip_sound,
            killed_blip_sound,
        );

        move |rl, thread| {
            let dt = rl.get_frame_time();
            // poor man's hot reload of shaders
            // circular clipping was a bit hard, as you can see
            if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
                post_processing_shaders_enabled = !post_processing_shaders_enabled;
                if let Ok(s) = rl
                    .load_shader(
                        thread,
                        None,
                        Some(&format!("{GLSL_DIR}/circular_clipping.fs")),
                    )
                    .inspect_err(|err| println!("circular: {}", err))
                {
                    shader_circular_clipping = s;
                }
                if post_processing_shaders_enabled {
                    if let Ok(s) = rl
                        .load_shader(thread, None, Some(&format!("{GLSL_DIR}/crt.fs")))
                        .inspect_err(|err| println!("crt: {}", err))
                    {
                        shader_crt = s;
                    }

                    if let Ok(s) = rl
                        .load_shader(thread, None, Some(&format!("{GLSL_DIR}/vignette.fs")))
                        .inspect_err(|err| println!("vig: {}", err))
                    {
                        shader_vignette = s;
                    }
                }
            }

            app_state.update(rl, dt);

            if matches!(app_state.phase, GamePhase::Play) {
                rl.hide_cursor();
                shader_circular_clipping
                    .set_shader_value(circle_center_loc, app_state.radar.center);
                shader_circular_clipping
                    .set_shader_value(circle_radius_loc, app_state.radar.radius);
                shader_circular_clipping.set_shader_value(screen_width_loc, screen_width as f32);
                shader_circular_clipping.set_shader_value(screen_height_loc, screen_height as f32);
            }

            rl.draw_texture_mode(thread, &mut app_render_texture, |mut d| {
                d.clear_background(Color::RAYWHITE);

                let title_text_width = d.measure_text(TITLE, FONT_SIZE);
                let gameover_text_width = d.measure_text(GAME_OVER, FONT_SIZE);
                app_state.draw(&mut d, title_text_width, gameover_text_width);
            });

            rl.draw_texture_mode(thread, &mut blips_render_texture, |mut d| {
                d.clear_background(Color::BLANK);
                if matches!(app_state.phase, GamePhase::Play) {
                    for blip in &app_state.blips {
                        blip.draw(&mut d);
                    }
                }
            });

            rl.draw_texture_mode(thread, &mut final_render_texture, |mut d| {
                d.draw_texture_rec(
                    &app_render_texture,
                    Rectangle::new(0., 0., screen_width as f32, -screen_height as f32),
                    Vector2::new(0., 0.),
                    Color::WHITE,
                );
                d.draw_shader_mode(&mut shader_circular_clipping, |mut d| {
                    d.draw_texture_rec(
                        &blips_render_texture,
                        Rectangle::new(0., 0., screen_width as f32, -screen_height as f32),
                        Vector2::new(0., 0.),
                        Color::WHITE,
                    );
                });
            });

            rl.draw_texture_mode(thread, &mut pass1_render_texture, |mut d| {
                if post_processing_shaders_enabled {
                    d.draw_shader_mode(&mut shader_crt, |mut d| {
                        d.draw_texture_rec(
                            &final_render_texture,
                            Rectangle::new(0., 0., screen_width as f32, -screen_height as f32),
                            Vector2::new(0., 0.),
                            Color::WHITE,
                        );
                    });
                } else {
                    d.draw_texture_rec(
                        &final_render_texture,
                        Rectangle::new(0., 0., screen_width as f32, -screen_height as f32),
                        Vector2::new(0., 0.),
                        Color::WHITE,
                    );
                }
            });

            rl.draw(thread, |mut d| {
                if post_processing_shaders_enabled {
                    d.draw_shader_mode(&mut shader_vignette, |mut d| {
                        d.draw_texture_rec(
                            &pass1_render_texture,
                            Rectangle::new(0., 0., screen_width as f32, -screen_height as f32),
                            Vector2::new(0., 0.),
                            Color::WHITE,
                        );
                    });
                } else {
                    d.draw_texture_rec(
                        &pass1_render_texture,
                        Rectangle::new(0., 0., screen_width as f32, -screen_height as f32),
                        Vector2::new(0., 0.),
                        Color::WHITE,
                    );
                }
            });
        }
    });

    Ok(())
}

struct Radar {
    center: Vector2,
    radius: f32,
}

impl Radar {
    fn new(center: Vector2, radius: f32) -> Self {
        Self { center, radius }
    }

    fn update(&mut self, rl: &mut RaylibHandle, _dt: f32) {
        self.center_area(rl.get_mouse_x(), rl.get_mouse_y());
    }

    fn center_area(&mut self, x: i32, y: i32) {
        self.center.x = x as f32;
        self.center.y = y as f32;
    }

    fn draw<T: RaylibDraw>(&self, d: &mut T) {
        d.clear_background(Color::RAYWHITE);

        d.draw_circle_v(self.center, self.radius, Color::DIMGRAY);

        d.draw_circle_lines_v(self.center, self.radius, Color::BLACK);
    }
}
