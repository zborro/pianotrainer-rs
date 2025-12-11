use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use midix::prelude::MetaMessage::*;
use midix::prelude::*;

#[derive(Clone, Debug)]
pub struct NoteBlock {
    pub octave: Octave,
    pub note: Note,
    pub key: Key,
    pub start_delta: u32,
    pub stop_delta: Option<u32>,
    pub start_time: u32,
    pub stop_time: Option<u32>,
    pub channel_number: u32,
}

impl fmt::Display for NoteBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "NoteBlock({}/{}@{})",
            self.octave, self.note, self.start_time
        )
    }
}

pub struct Channel {
    current_delta: u32,
    current_time: u32,
    note_blocks: Vec<NoteBlock>,
}

pub struct Song {
    note_blocks: Vec<Vec<NoteBlock>>,
}

impl Song {
    fn should_include(&self, from_time: u32, to_time: u32, group: &Vec<NoteBlock>) -> bool {
        let mut result = false;
        for block in group {
            if (block.start_time > from_time && block.start_time < to_time)
                || (block.stop_time.unwrap_or(0) > from_time
                    && block.stop_time.unwrap_or(0) < to_time)
                || (block.start_time < from_time && block.stop_time.unwrap_or(0) > to_time)
            {
                result = true;
            }
        }

        result
    }

    pub fn range(&self, from_time: u32, to_time: u32) -> &[Vec<NoteBlock>] {
        let mut from_ix = self.note_blocks.len();
        let mut to_ix = 0;

        for (ix, group) in self.note_blocks.iter().enumerate() {
            if self.should_include(from_time, to_time, group) {
                if ix < from_ix {
                    from_ix = ix;
                }
                if ix > to_ix {
                    to_ix = ix;
                }
            }
        }

        &self.note_blocks[from_ix..to_ix]
    }

    fn time_offset_to_index(&self, from_time: u32) -> i32 {
        let mut index = -1;
        for (i, group) in self.note_blocks.iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            if group.first().unwrap().start_time > from_time {
                index = i as i32;
                break;
            }
        }
        index
    }

    pub fn next(&self, from_time: u32) -> Option<&[NoteBlock]> {
        let index = self.time_offset_to_index(from_time);

        if index > 0 {
            Some(&self.note_blocks[index as usize])
        } else {
            None
        }
    }

    pub fn prev(&self, from_time: u32) -> Option<&[NoteBlock]> {
        let index = std::cmp::max(0, self.time_offset_to_index(from_time) - 2);
        Some(&self.note_blocks[index as usize])
    }

    pub fn load(path: &Path) -> Self {
        let display = path.display();

        let mut file = match File::open(path) {
            Err(why) => panic!("could not open {}: {}", display, why),
            Ok(file) => file,
        };

        let mut cur_us_per_quarter_note = meta::Tempo::default().micros_per_quarter_note();
        let mut ticks_per_quarter_note = 48;

        let mut channels: HashMap<u32, Channel> = HashMap::new();

        let mut buf: Vec<u8> = vec![];

        if let Err(why) = file.read_to_end(&mut buf) {
            panic!("error reading from file {}: {}", display, why);
        }

        let mut reader = Reader::from_byte_slice(&buf);

        loop {
            match reader.read_event() {
                Err(why) => panic!("failed to process event from midi: {}", why),
                Ok(FileEvent::Header(header)) => {
                    ticks_per_quarter_note =
                        header.timing().ticks_per_quarter_note().unwrap_or(48) as u32
                }
                Ok(FileEvent::Track(_)) => {}
                Ok(FileEvent::TrackEvent(track_event)) => {
                    let dt = track_event.delta_ticks();
                    let dt2 = dt
                        * ((cur_us_per_quarter_note as f32 / ticks_per_quarter_note as f32) as u32);

                    match track_event.event() {
                        TrackMessage::ChannelVoice(cv) => {
                            let channel = cv.channel();

                            let channel_obj =
                                &mut channels.entry(channel as u32).or_insert(Channel {
                                    current_delta: 0,
                                    current_time: 0,
                                    note_blocks: vec![],
                                });
                            channel_obj.current_delta += dt;
                            channel_obj.current_time += dt2;

                            match cv.event() {
                                VoiceEvent::NoteOn { key, .. } => {
                                    channel_obj.note_blocks.push(NoteBlock {
                                        octave: key.octave(),
                                        note: key.note(),
                                        key: key.clone(),
                                        start_delta: channel_obj.current_delta,
                                        stop_delta: None,
                                        start_time: channel_obj.current_time,
                                        stop_time: None,
                                        channel_number: channel as u32,
                                    });
                                    // NoteOn w/ velocity=0 is NoteOff?
                                }
                                VoiceEvent::NoteOff { key, .. } => {
                                    for block in channel_obj.note_blocks.iter_mut().rev() {
                                        if block.stop_delta.is_none()
                                            && block.octave == key.octave()
                                            && block.note == key.note()
                                        {
                                            block.stop_delta = Some(channel_obj.current_delta);
                                            block.stop_time = Some(channel_obj.current_time);
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
                        TrackMessage::Meta(meta_event) => match meta_event {
                            Tempo(tempo_event) => {
                                cur_us_per_quarter_note = tempo_event.micros_per_quarter_note();
                            }
                            TimeSignature(time_signature_event) => {
                                println!(
                                    "Time signature = num={} den={} cpc={} 32s={}",
                                    time_signature_event.num(),
                                    time_signature_event.den(),
                                    time_signature_event.clocks_per_click(),
                                    time_signature_event.notated_32nds_per_24_clocks()
                                );
                            }
                            _ => (),
                        },
                    }
                }
                Ok(FileEvent::EOF) => break,
                Ok(_) => (),
            }
        }

        let mut groups = vec![];

        for ch in channels.values() {
            groups.extend(ch.note_blocks.to_vec());
        }

        groups.sort_by(|a, b| a.start_delta.partial_cmp(&b.start_delta).unwrap());
        let chunk_by = groups.chunk_by(|a, b| a.start_delta == b.start_delta);

        Song {
            note_blocks: chunk_by
                .map(|x| x.to_vec())
                .filter(|v| !v.is_empty())
                .collect(),
        }
    }
}
