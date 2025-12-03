use macroquad::prelude::*;
use std::path::PathBuf;
use clap::Parser;

mod song;
mod screen;

#[derive(Parser)]
struct Cli {
    midi_path: PathBuf,
}

#[macroquad::main("zborro-piano-trainer")]
async fn main() {
    let args = Cli::parse();
    println!("{:?}", args.midi_path.to_str());

    if !args.midi_path.exists() {
        panic!("passed MIDI file does not exist!");
    }

    let song = song::load_song(args.midi_path.as_path());

    let mut last_screen_width = screen_width();
    let piano_screen_handle = scene::add_node(screen::PianoScreen::new(song));

    let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., screen_width(), screen_height()));
    scene::set_camera(0, Some(camera));

    loop {
        if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            return;
        }

        if screen_width() != last_screen_width {
            scene::get_node(piano_screen_handle).on_screen_resize();

            camera = Camera2D::from_display_rect(Rect::new(0., 0., screen_width(), screen_height()));
            scene::set_camera(0, Some(camera));

            last_screen_width = screen_width();
        }

        next_frame().await
    }
}
