use clap::Parser;
use macroquad::prelude::*;
use midir::{Ignore, MidiInput};
use std::error::Error;
use std::path::PathBuf;
use std::thread;
use core::time;

mod screen;
mod song;

#[derive(Parser)]
struct Cli {
    midi_path: PathBuf,
    #[arg(long = "midi-port")]
    midi_port: String,
}

#[macroquad::main("zborro-piano-trainer")]
async fn main() {
    let args = Cli::parse();
    println!("{:?}", args.midi_path.to_str());

    if !args.midi_path.exists() {
        panic!("passed MIDI file does not exist!");
    }

    env_logger::init();
    match run(args.midi_path, args.midi_port).await {
        Ok(_) => (),
        Err(why) => println!("Error: {}", why),
    }
}

async fn run(midi_path: PathBuf, midi_port: String) -> Result<(), Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    let mut last_screen_width = screen_width();

    let song = song::load_song(midi_path.as_path());
    let piano_screen_handle = scene::add_node(screen::PianoScreen::new(song));

    let in_port = midi_in.find_port_by_id(midi_port).unwrap();

    let _conn_in = midi_in.connect(
        &in_port,
        "midir-read-input",
        move |stamp, message, _| {
            println!("{}: {:?} (len = {})", stamp, message, message.len());
            loop {
                let node = scene::try_get_node(piano_screen_handle);
                if node.is_some() {
                    node.unwrap().on_piano_key(123);
                    break;
                }
                else {
                    thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        },
        (),
    )?;

    let mut camera =
        Camera2D::from_display_rect(Rect::new(0., 0., screen_width(), screen_height()));
    scene::set_camera(0, Some(camera));

    loop {
        if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::C) {
            scene::get_node(piano_screen_handle).on_piano_key(44);
        }

        if screen_width() != last_screen_width {
            scene::get_node(piano_screen_handle).on_screen_resize();

            camera =
                Camera2D::from_display_rect(Rect::new(0., 0., screen_width(), screen_height()));
            scene::set_camera(0, Some(camera));

            last_screen_width = screen_width();
        }

        next_frame().await
    }

    scene::clear();

    Ok(())
}
