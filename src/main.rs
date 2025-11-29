use macroquad::prelude::*;
use midix::prelude::*;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

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

    let mut buf: Vec<u8> = vec![];

    if let Err(why) = file.read_to_end(&mut buf) {
        panic!("error reading from file {}: {}", display, why);
    }

    println!("read the file. its len: {}", buf.len());

    let mut reader = Reader::from_byte_slice(&buf);

    let mut header: Option<RawHeaderChunk> = None;

    loop {
        match reader.read_event() {
            Err(why) => panic!("failed to process event from midi: {}", why),
            Ok(FileEvent::Header(hdr)) => {
                println!("header found");
                header = Some(hdr);
            }
            Ok(FileEvent::Track(track)) => {
                println!("track found");
            }
            Ok(FileEvent::TrackEvent(track_event)) => {
                let dt = track_event.delta_ticks();

                match track_event.event() {
                    TrackMessage::ChannelVoice(cv) => {
                        let channel = cv.channel();

                        match cv.event() {
                            VoiceEvent::NoteOn { key, .. } => {
                                println!("{} | ch:{} note on {}", dt, channel, key);
                            }
                            VoiceEvent::NoteOff { key, .. } => {
                                println!("{} | ch:{} note off {}", dt, channel, key);
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

    loop {
        let white_piano_key_height = 200.;
        let white_piano_key_width = (screen_width() / ((num_white_keys + 1) as f32)) - 2.;
        let black_piano_key_height = 130.;
        let black_piano_key_width = white_piano_key_width * 0.5;

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
        next_frame().await
    }
}
