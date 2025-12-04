use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use midix::prelude::*;

#[derive(Clone)]
pub struct NoteBlock {
    pub octave: Octave,
    pub note: Note,
    pub key: Key,
    pub start_time: u32,
    pub stop_time: Option<u32>,
}

pub struct Channel {
    curtime: u32,
    pub note_blocks: Vec<NoteBlock>,
}

pub struct Song {
    pub channels: HashMap<u32, Channel>,
}

impl Song {
    pub fn get_note_blocks_ordered(&self) -> Vec<Vec<NoteBlock>> {
        let mut results = vec![];

        for ch in self.channels.values() {
            results.extend(ch.note_blocks.to_vec());
        }

        results.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        let chunk_by = results.chunk_by(|a, b| a.start_time == b.start_time);
        chunk_by.map(|x| x.to_vec()).collect()
    }
}

pub fn load_song(path: &Path) -> Song {
    let display = path.display();

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

    let mut reader = Reader::from_byte_slice(&buf);

    let mut _header: Option<RawHeaderChunk> = None;

    loop {
        match reader.read_event() {
            Err(why) => panic!("failed to process event from midi: {}", why),
            Ok(FileEvent::Header(hdr)) => {
                _header = Some(hdr);
            }
            Ok(FileEvent::Track(_)) => {}
            Ok(FileEvent::TrackEvent(track_event)) => {
                let dt = track_event.delta_ticks();

                match track_event.event() {
                    TrackMessage::ChannelVoice(cv) => {
                        let channel = cv.channel();

                        let channel_obj = song.channels.entry(channel as u32).or_insert(Channel {
                            curtime: 0,
                            note_blocks: vec![],
                        });
                        channel_obj.curtime += dt;

                        match cv.event() {
                            VoiceEvent::NoteOn { key, .. } => {
                                channel_obj.note_blocks.push(NoteBlock {
                                    octave: key.octave(),
                                    note: key.note(),
                                    key: key.clone(),
                                    start_time: channel_obj.curtime,
                                    stop_time: None,
                                });
                            }
                            VoiceEvent::NoteOff { key, .. } => {
                                for block in channel_obj.note_blocks.iter_mut().rev() {
                                    if block.stop_time.is_none()
                                        && block.octave == key.octave()
                                        && block.note == key.note()
                                    {
                                        block.stop_time = Some(channel_obj.curtime);
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

    song
}
