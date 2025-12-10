use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use macroquad::experimental::scene::{Node, RefMut};
use macroquad::prelude::*;
use midix::prelude::Key;

use crate::song;

#[derive(Hash, Eq, PartialEq)]
pub struct KeyWithTimestamp {
    key: Key,
    timestamp: SystemTime,
}

pub struct ActiveKeysHistory {
    history: HashSet<KeyWithTimestamp>,
}

impl ActiveKeysHistory {
    pub fn new() -> Self {
        Self {
            history: HashSet::new(),
        }
    }

    pub fn insert(&mut self, key: Key) {
        self.history.insert(KeyWithTimestamp {
            key,
            timestamp: SystemTime::now(),
        });
    }

    pub fn autoclean(&mut self) {
        self.history
            .retain(|e| e.timestamp.elapsed().is_ok_and(|v| v.as_secs() < 2));
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }

    pub fn get(&self) -> HashSet<Key> {
        self.history.iter().map(|e| e.key).collect()
    }

    pub fn len(&self) -> usize {
        self.get().len()
    }
}

#[derive(Eq, PartialEq)]
pub enum GameMode {
    Play,
    LearnBlocking,
}

pub struct PianoScreen {
    song: song::Song,
    mode: GameMode,
    play: bool,
    time_offset: f32,
    time_offset_y: f32,
    num_white_keys: u32,
    piano_key_margin: f32,
    white_piano_key_width: f32,
    white_piano_key_height: f32,
    black_piano_key_width: f32,
    black_piano_key_height: f32,
    midi_render_target: RenderTarget,
    midi_target_cam: Camera2D,
    render_debug_extra: bool,
    text_texture_cache: HashMap<String, Texture2D>,
    pixels_per_second: f32,
    default_pixels_per_second: f32,
    paused_on_block_group: u32,
    awaiting_piano_input: bool,
    awaiting_keys: Option<HashSet<Key>>,
    active_piano_keys: HashSet<Key>,
    active_piano_keys_history: ActiveKeysHistory,
}

impl PianoScreen {
    pub fn recalculate(&mut self, width: f32, _height: f32) {
        self.white_piano_key_height = 200.;
        self.white_piano_key_width = (width / ((self.num_white_keys + 1) as f32)) - 2.;
        self.black_piano_key_height = 130.;
        self.black_piano_key_width = self.white_piano_key_width * 0.5;

        self.midi_render_target = render_target(
            screen_width() as u32,
            (screen_height() - self.white_piano_key_height) as u32,
        );
        self.midi_target_cam = Camera2D::from_display_rect(Rect::new(
            0.,
            0.,
            screen_width(),
            screen_height() - self.white_piano_key_height,
        ));
        self.midi_target_cam.render_target = Some(self.midi_render_target.clone());
    }

    pub fn new(song: song::Song) -> PianoScreen {
        let mut ps = PianoScreen {
            song,
            mode: GameMode::Play,
            play: false,
            time_offset: 0.,
            time_offset_y: 0.,
            num_white_keys: 52,
            piano_key_margin: 1.,
            white_piano_key_width: 0.,
            white_piano_key_height: 0.,
            black_piano_key_width: 0.,
            black_piano_key_height: 0.,
            midi_render_target: render_target(screen_width() as u32, screen_height() as u32),
            midi_target_cam: Camera2D::from_display_rect(Rect::new(0., 0., 100., 100.)),
            render_debug_extra: false,
            text_texture_cache: HashMap::new(),
            pixels_per_second: 400.,
            default_pixels_per_second: 400.,
            paused_on_block_group: 0,
            awaiting_piano_input: false,
            awaiting_keys: None,
            active_piano_keys: HashSet::new(),
            active_piano_keys_history: ActiveKeysHistory::new(),
        };
        ps.recalculate(screen_width(), screen_height());
        ps
    }

    pub fn reset(&mut self) {
        self.mode = GameMode::Play;
        self.time_offset = 0.;
        self.time_offset_y = 0.;
        self.pixels_per_second = self.default_pixels_per_second;
        self.paused_on_block_group = 0;
        self.awaiting_piano_input = false;
        self.awaiting_keys = None;
        self.active_piano_keys.clear();
        self.active_piano_keys_history.clear();
    }

    pub fn on_screen_resize(&mut self) {
        self.recalculate(screen_width(), screen_height());
    }

    fn draw_piano_keyboard(&self) {
        clear_background(GRAY);

        let key_byte_offset = 21;

        let num_piano_keys = 89;
        let c1_offset = (self.white_piano_key_width + 2.) * 2. + 1.;
        let octave_w = (self.white_piano_key_width + 3.) * 7.;

        for black_u32 in 0..2u32 {
            let black = black_u32 == 1;
            for i in 0..num_piano_keys {
                let key = Key::from_databyte((key_byte_offset + i) as u8).unwrap();
                if self.is_key_white(key) == black {
                    continue;
                }

                let octave_offset = (key.octave().value() - 1) as f32 * octave_w;
                let note_offset = self.calc_note_offset(key);
                let color = if self.active_piano_keys.contains(&key) {
                    RED
                } else {
                    if black { BLACK } else { WHITE }
                };

                draw_rectangle(
                    c1_offset + octave_offset + note_offset,
                    if black {
                        self.white_piano_key_height - self.black_piano_key_height
                    } else {
                        0.
                    },
                    if black {
                        self.black_piano_key_width
                    } else {
                        self.white_piano_key_width
                    },
                    if black {
                        self.black_piano_key_height
                    } else {
                        self.white_piano_key_height
                    },
                    color,
                );
            }
        }
    }

    fn get_note_block_color(&self, channel_number: u32, sharp: bool) -> Color {
        match sharp {
            true => match channel_number {
                0 => GREEN,
                1 => BLUE,
                2 => PURPLE,
                3 => YELLOW,
                _ => GRAY,
            },
            false => match channel_number {
                0 => DARKGREEN,
                1 => DARKBLUE,
                2 => DARKPURPLE,
                3 => ORANGE,
                _ => GRAY,
            },
        }
    }

    fn is_key_white(&self, key: Key) -> bool {
        match key.byte() % 12 {
            0 | 2 | 4 | 5 | 7 | 9 | 11 => true,
            _ => false,
        }
    }

    fn calc_note_offset(&self, key: Key) -> f32 {
        (self.white_piano_key_width + 3.)
            * match key.byte() % 12 {
                0 => 0.,    // C
                1 => 0.75,  // C#
                2 => 1.,    // D
                3 => 1.75,  // D#
                4 => 2.,    // E
                5 => 3.,    // F
                6 => 3.75,  // F#
                7 => 4.,    // G
                8 => 4.75,  // G#
                9 => 5.,    // A
                10 => 5.75, // A#
                11 => 6.,   // B
                _ => 0.,
            }
    }

    fn render_inverse_text(&self, text: &str) -> Texture2D {
        let text_size = measure_text(text, None, 16, 1.);
        let text_render_target =
            render_target((text_size.width) as u32, (text_size.height * 2.) as u32);

        let mut camera =
            Camera2D::from_display_rect(Rect::new(0., 0., text_size.width, text_size.height * 2.));
        camera.render_target = Some(text_render_target.clone());

        push_camera_state();

        set_camera(&camera);
        clear_background(BLACK);
        draw_text(text, 0., 12., 16., RED);

        set_default_camera();
        pop_camera_state();

        text_render_target.texture.clone()
    }

    fn draw_song_timeline(&mut self) {
        set_camera(&self.midi_target_cam);
        clear_background(BLACK);

        let c1_offset = (self.white_piano_key_width + 2.) * 2. + 1.;
        let octave_w = (self.white_piano_key_width + 3.) * 7.;

        for i in 0..8 {
            draw_line(
                c1_offset + i as f32 * octave_w,
                0.,
                c1_offset + i as f32 * octave_w,
                screen_height(),
                1.,
                GRAY,
            );
        }

        let song_iter = &self.song.get_iterator();

        let from_time = self.time_offset as u32 * 1_000_000;
        let to_time =
            from_time + (((screen_height() / self.pixels_per_second) * 1_000_000.) * 1.5) as u32;

        for chunk in song_iter.range(from_time, to_time) {
            for block in chunk {
                let octave_offset = (block.octave.value() - 1) as f32 * octave_w;
                let note_offset = self.calc_note_offset(block.key);

                let block_x = c1_offset + octave_offset + note_offset;
                let block_y = ((block.start_time as f32) / 1_000_000.) * self.pixels_per_second;
                let block_w = if block.key.is_sharp() {
                    self.black_piano_key_width
                } else {
                    self.white_piano_key_width
                };
                let block_h = (((block.stop_time.unwrap() - block.start_time) as f32) / 1_000_000.)
                    * self.pixels_per_second;

                draw_rectangle(
                    block_x,
                    block_y - self.time_offset_y,
                    block_w,
                    block_h,
                    self.get_note_block_color(block.channel_number, !block.note.is_flat()),
                );

                if self.render_debug_extra {
                    let line_y = ((block.start_time as f32) / 1_000_000.) * self.pixels_per_second;
                    let line_xo = if block.key.is_sharp() {
                        self.black_piano_key_width / 2.
                    } else {
                        self.white_piano_key_width / 2.
                    };
                    let line_x = c1_offset + octave_offset + note_offset + line_xo;
                    let line_h = ((block.stop_time.unwrap() - block.start_time) as f32
                        / 1_000_000.)
                        * self.pixels_per_second;

                    draw_line(
                        line_x,
                        line_y - self.time_offset_y,
                        line_x,
                        line_y - self.time_offset_y + line_h,
                        2.,
                        RED,
                    );

                    let texture_key: String = format!("{}", block.start_delta).to_string();
                    if !self.text_texture_cache.contains_key(&texture_key) {
                        let texttex = self.render_inverse_text(&format!(
                            "{} / {}ms",
                            block.start_delta,
                            (block.start_time as f64 / 1_000.) as u32
                        ));
                        self.text_texture_cache
                            .insert(texture_key.to_string(), texttex.clone());
                    }
                    let texttex = self.text_texture_cache.get(&texture_key).unwrap();
                    draw_texture_ex(
                        texttex,
                        line_x + 5.,
                        line_y - self.time_offset_y - 5.,
                        WHITE,
                        DrawTextureParams {
                            ..Default::default()
                        },
                    );
                }
            }
        }

        set_default_camera();

        draw_texture_ex(
            &self.midi_render_target.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(
                    screen_width() as f32,
                    (screen_height() - self.white_piano_key_height) as f32,
                )),
                ..Default::default()
            },
        );

        draw_text(&format!("T: {}s", self.time_offset), 10., 40., 32., RED);
        draw_text(
            &format!("#pkd: {}", self.active_piano_keys.len()),
            10.,
            70.,
            32.,
            RED,
        );
        draw_text(
            &format!("#pkdH: {}", self.active_piano_keys_history.len()),
            10.,
            100.,
            32.,
            RED,
        );

        draw_text(
            &format!(
                "mode: {}",
                match self.mode {
                    GameMode::Play => "play",
                    GameMode::LearnBlocking => "blocking-learn",
                }
            ),
            10.,
            130.,
            32.,
            RED,
        );
    }

    pub fn toggle_play(&mut self) {
        self.play = !self.play;
    }

    pub fn update(&mut self, frame_time: f32) {
        let note_block_groups = self.song.get_note_blocks_ordered();
        self.active_piano_keys_history.autoclean();

        if (self.paused_on_block_group as usize) < note_block_groups.len() {
            let next_group = note_block_groups
                .get(self.paused_on_block_group as usize)
                .unwrap();
            let next_group_start_time = next_group.first().unwrap().start_time;

            // todo: make sure that this doesn't get called more than once
            if (self.time_offset * 1_000_000.) as u32 >= next_group_start_time {
                self.awaiting_piano_input = true;
                self.awaiting_keys =
                    Some(next_group.iter().map(|b| b.key).collect::<HashSet<Key>>());
            }
        }

        if self.play
            && (self.mode == GameMode::Play
                || (self.mode == GameMode::LearnBlocking && !self.awaiting_piano_input))
        {
            self.time_offset += frame_time;
            self.time_offset_y += frame_time * self.pixels_per_second;
        }
    }

    pub fn on_piano_key_down(&mut self, key: Key) {
        self.active_piano_keys.insert(key);
        self.active_piano_keys_history.insert(key);
        if self.awaiting_piano_input {
            if self.active_piano_keys == self.awaiting_keys.clone().unwrap() {
                self.awaiting_piano_input = false;
                self.paused_on_block_group += 1;

                // clear to make sure that successive piano key strokes
                // won't be polluted with previous ones
                self.active_piano_keys_history.clear();
            }
        }
    }

    pub fn on_piano_key_up(&mut self, key: Key) {
        self.active_piano_keys.remove(&key);
    }

    pub fn zoom_out(&mut self) {
        self.pixels_per_second -= 10.;
    }

    pub fn zoom_in(&mut self) {
        self.pixels_per_second += 10.;
    }

    pub fn zoom_default(&mut self) {
        self.pixels_per_second = self.default_pixels_per_second;
    }

    pub fn set_mode(&mut self, mode: GameMode) {
        self.mode = mode;
    }
}

impl Node for PianoScreen {
    fn ready(_node: RefMut<Self>) {}

    fn draw(mut node: RefMut<Self>) {
        node.draw_piano_keyboard();
        node.draw_song_timeline();
    }

    fn update(mut node: RefMut<Self>) {
        if is_key_pressed(KeyCode::Space) {
            node.toggle_play();
        }

        node.update(get_frame_time());
    }
}
