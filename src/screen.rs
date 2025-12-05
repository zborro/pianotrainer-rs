use std::collections::{HashMap, HashSet};

use macroquad::experimental::scene::{Node, RefMut};
use macroquad::prelude::*;

use crate::song;

pub struct PianoScreen {
    song: song::Song,
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
        };
        ps.recalculate(screen_width(), screen_height());
        ps
    }

    pub fn on_screen_resize(&mut self) {
        self.recalculate(screen_width(), screen_height());
    }

    fn draw_piano_keyboard(&self) {
        clear_background(GRAY);

        let black_key_positions: HashSet<i32> = HashSet::from([
            0, 2, 3, 5, 6, 7, 9, 10, 12, 13, 14, 16, 17, 19, 20, 21, 23, 24, 26, 27, 28, 30, 31,
            33, 34, 35, 37, 38, 40, 41, 42, 44, 45, 47, 48, 49,
        ]);

        for i in 0..(self.num_white_keys as i32) {
            let x = (i as f32) * (self.white_piano_key_width + self.piano_key_margin)
                + (i as f32 * self.piano_key_margin * 2.);
            draw_rectangle(
                x,
                0.,
                self.white_piano_key_width,
                self.white_piano_key_height,
                WHITE,
            );

            if black_key_positions.contains(&((i - 1) as i32)) {
                draw_rectangle(
                    x - (self.black_piano_key_width / 2.) - 2.,
                    self.white_piano_key_height - self.black_piano_key_height,
                    self.black_piano_key_width,
                    self.black_piano_key_height,
                    BLACK,
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

    fn calc_note_offset(&self, block: &song::NoteBlock) -> f32 {
        (self.white_piano_key_width + 3.)
            * match block.key.byte() % 12 {
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

        let ordered_block_groups = &self.song.get_note_blocks_ordered();

        for chunk in ordered_block_groups {
            for block in chunk {
                let octave_offset = (block.octave.value() - 1) as f32 * octave_w;
                let note_offset = self.calc_note_offset(block);

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
                        self.text_texture_cache.insert(texture_key.to_string(), texttex.clone());
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

        draw_text(&format!("t: {}s", self.time_offset), 10., 40., 32., RED);
    }

    pub fn toggle_play(&mut self) {
        self.play = !self.play;
    }

    pub fn update(&mut self, frame_time: f32) {
        if self.play {
            self.time_offset += frame_time;
            self.time_offset_y += frame_time * self.pixels_per_second;
        }
    }

    pub fn on_piano_key(&mut self, key: u32) {
        println!("piano key event! {}", key);
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
