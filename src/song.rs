use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use midix::prelude::MetaMessage::*;
use midix::prelude::*;

#[derive(Clone)]
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

pub struct Channel {
    current_delta: u32,
    current_time: u32,
    pub note_blocks: Vec<NoteBlock>,
}

pub struct Song {
    pub cur_us_per_quarter_note: u32,
    pub ticks_per_quarter_note: u32,
    pub channels: HashMap<u32, Channel>,
}

pub struct SongIterator {
    note_blocks: Vec<Vec<NoteBlock>>,
}

impl SongIterator {
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
}

impl Song {
    pub fn get_note_blocks_ordered(&self) -> Vec<Vec<NoteBlock>> {
        let mut results = vec![];

        for ch in self.channels.values() {
            results.extend(ch.note_blocks.to_vec());
        }

        results.sort_by(|a, b| a.start_delta.partial_cmp(&b.start_delta).unwrap());

        let chunk_by = results.chunk_by(|a, b| a.start_delta == b.start_delta);
        chunk_by.map(|x| x.to_vec()).filter(|v| v.len() > 0).collect()
    }

    pub fn get_iterator(&self) -> SongIterator {
        SongIterator {
            note_blocks: self.get_note_blocks_ordered(),
        }
    }

    pub fn us_per_tick(&self) -> u32 {
        (self.cur_us_per_quarter_note as f32 / self.ticks_per_quarter_note as f32) as u32
    }

    pub fn load(path: &Path) -> Self {
        let display = path.display();

        let mut file = match File::open(path) {
            Err(why) => panic!("could not open {}: {}", display, why),
            Ok(file) => file,
        };

        let mut song: Self = Song {
            cur_us_per_quarter_note: meta::Tempo::default().micros_per_quarter_note(),
            ticks_per_quarter_note: 48,
            channels: HashMap::new(),
        };

        let mut buf: Vec<u8> = vec![];

        if let Err(why) = file.read_to_end(&mut buf) {
            panic!("error reading from file {}: {}", display, why);
        }

        let mut reader = Reader::from_byte_slice(&buf);

        loop {
            match reader.read_event() {
                Err(why) => panic!("failed to process event from midi: {}", why),
                Ok(FileEvent::Header(header)) => {
                    song.ticks_per_quarter_note =
                        header.timing().ticks_per_quarter_note().unwrap_or(48) as u32
                }
                Ok(FileEvent::Track(_)) => {}
                Ok(FileEvent::TrackEvent(track_event)) => {
                    let dt = track_event.delta_ticks();
                    let dt2 = dt * song.us_per_tick();

                    match track_event.event() {
                        TrackMessage::ChannelVoice(cv) => {
                            let channel = cv.channel();

                            let channel_obj =
                                &mut song.channels.entry(channel as u32).or_insert(Channel {
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
                                song.cur_us_per_quarter_note = tempo_event.micros_per_quarter_note();
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

        song
    }
}
