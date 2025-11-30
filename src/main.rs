use macroquad::prelude::*;
use midix::prelude::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

struct NoteBlock {
    octave: Octave,
    note: Note,
    key: Key,
    start_time: u32,
    stop_time: Option<u32>,
}

struct Channel {
    number: u32,
    curtime: u32,
    note_blocks: Vec<NoteBlock>,
}

struct Song {
    channels: HashMap<u32, Channel>,
}

#[macroquad::main("ZborroPianoApp")]
async fn main() {
    if env::args().len() != 2 {
        panic!("ERROR : wrong number of arguments! Expected file as arg");
    }

    let path_string = env::args().nth(1).unwrap();
    let path = Path::new(&path_string);
    let display = path.display();

    println!("trying to open file {display}");
    let mut file = match File::open(path) {
        Err(why) => panic!("could not open {}: {}", display, why),
        Ok(file) => file,
    };

    let mut song: Song = Song {
        channels: HashMap::new(),
    };

    let mut buf: Vec<u8> = vec![];

    if let Err(why) = file.read_to_end(&mut buf) {
        panic!("error reading from file {}: {}", display, why);
    }

    println!("read the file. its len: {}", buf.len());

    let mut reader = Reader::from_byte_slice(&buf);

    let mut _header: Option<RawHeaderChunk> = None;

    loop {
        match reader.read_event() {
            Err(why) => panic!("failed to process event from midi: {}", why),
            Ok(FileEvent::Header(hdr)) => {
                println!("header found");
                _header = Some(hdr);
            }
            Ok(FileEvent::Track(_)) => {
                println!("track found");
            }
            Ok(FileEvent::TrackEvent(track_event)) => {
                let dt = track_event.delta_ticks();

                match track_event.event() {
                    TrackMessage::ChannelVoice(cv) => {
                        let channel = cv.channel();

                        let channelObj = song.channels.entry(channel as u32).or_insert(Channel {
                            number: channel as u32,
                            curtime: 0,
                            note_blocks: vec![],
                        });
                        channelObj.curtime += dt;

                        match cv.event() {
                            VoiceEvent::NoteOn { key, .. } => {
                                println!("{} | ch:{} note on {}", dt, channel, key);
                                channelObj.note_blocks.push(NoteBlock {
                                    octave: key.octave(),
                                    note: key.note(),
                                    key: key.clone(),
                                    start_time: channelObj.curtime,
                                    stop_time: None,
                                });
                            }
                            VoiceEvent::NoteOff { key, .. } => {
                                println!("{} | ch:{} note off {}", dt, channel, key);

                                for block in channelObj.note_blocks.iter_mut().rev() {
                                    if block.stop_time.is_none()
                                        && block.octave == key.octave()
                                        && block.note == key.note()
                                    {
                                        block.stop_time = Some(channelObj.curtime);
                                    }
                                }
                            }
                            VoiceEvent::Aftertouch { .. } => {}
                            VoiceEvent::ControlChange { .. } => (),
                            VoiceEvent::ProgramChange { .. } => (),
                            VoiceEvent::ChannelPressureAfterTouch { .. } => (),
                            VoiceEvent::PitchBend { .. } => (),
                        }
                    }
                    TrackMessage::SystemExclusive(_) => {}
                    TrackMessage::Meta(_) => {}
                }
            }
            Ok(FileEvent::EOF) => break,
            Ok(_) => (),
        }
    }

    let num_white_keys = 52;
    let piano_key_margin = 1.;

    let black_key_positions: HashSet<i32> = HashSet::from([
        0, 2, 3, 5, 6, 7, 9, 10, 12, 13, 14, 16, 17, 19, 20, 21, 23, 24, 26, 27, 28, 30, 31, 33,
        34, 35, 37, 38, 40, 41, 42, 44, 45, 47, 48, 49,
    ]);

    let white_piano_key_height = 200.;

    let mut last_screen_width = screen_width();

    let mut render_target_0 = render_target(screen_width() as u32, (screen_height() - 200.) as u32);
    let mut midi_target_cam = Camera2D::from_display_rect(Rect::new(
        0.,
        0.,
        screen_width(),
        screen_height() - white_piano_key_height,
    ));
    midi_target_cam.render_target = Some(render_target_0.clone());

    let mut time_offset_y = 0.;
    let mut play = false;
    let _pixels_per_second = 100.;

    loop {
        if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            return;
        }

        if is_key_pressed(KeyCode::Space) {
            play = !play;
        }

        if screen_width() != last_screen_width {
            render_target_0 = render_target(screen_width() as u32, (screen_height() - 200.) as u32);
            midi_target_cam = Camera2D::from_display_rect(Rect::new(
                0.,
                0.,
                screen_width(),
                screen_height() - white_piano_key_height,
            ));
            midi_target_cam.render_target = Some(render_target_0.clone());
            last_screen_width = screen_width();
        }

        if play {
            time_offset_y += get_frame_time() * 200.;
        }

        let white_piano_key_width = (screen_width() / ((num_white_keys + 1) as f32)) - 2.;
        let black_piano_key_height = 130.;
        let black_piano_key_width = white_piano_key_width * 0.5;

        set_camera(&midi_target_cam);

        clear_background(BLACK);

        let c1_offset = (white_piano_key_width + 2.) * 2. + 1.;
        let octave_w = (white_piano_key_width + 3.) * 7.;

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

        for (channelNumber, channelObj) in &song.channels {
            for itm in channelObj.note_blocks.iter() {
                let octave_offset = (itm.octave.value() - 1) as f32 * octave_w;

                let note_offset = (white_piano_key_width + 3.)
                    * match itm.key.byte() % 12 {
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
                    };

                let block_x = c1_offset + octave_offset + note_offset;
                let block_y = (itm.start_time as f32) / 10.;
                let block_w = if itm.key.is_sharp() {
                    black_piano_key_width
                } else {
                    white_piano_key_width
                };
                let block_h = (itm.stop_time.unwrap() - itm.start_time) as f32 / 10.;

                let sharp_color = match channelNumber {
                    0 => GREEN,
                    1 => BLUE,
                    2 => PURPLE,
                    3 => YELLOW,
                    _ => GRAY,
                };

                let flat_color = match channelNumber {
                    0 => DARKGREEN,
                    1 => DARKBLUE,
                    2 => DARKPURPLE,
                    3 => ORANGE,
                    _ => GRAY,
                };


                draw_rectangle(block_x, block_y - time_offset_y, block_w, block_h, if (itm.note.is_flat()) { flat_color } else { sharp_color });
            }
        }

        set_default_camera();

        clear_background(GRAY);

        for i in 0..num_white_keys {
            let x = (i as f32) * (white_piano_key_width + piano_key_margin)
                + (i as f32 * piano_key_margin * 2.);
            let y = screen_height() - white_piano_key_height;
            draw_rectangle(x, y, white_piano_key_width, white_piano_key_height, WHITE);

            if black_key_positions.contains(&(i - 1)) {
                draw_rectangle(
                    x - (black_piano_key_width / 2.) - 2.,
                    y,
                    black_piano_key_width,
                    black_piano_key_height,
                    BLACK,
                );
            }
        }

        draw_texture_ex(
            &render_target_0.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width() as f32, (screen_height() - 200.) as f32)),
                ..Default::default()
            },
        );

        next_frame().await
    }
}
