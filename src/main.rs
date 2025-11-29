use midix::prelude::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn main() {
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
}
