use macroquad::prelude::*;
use std::env;
use std::path::Path;

mod song;
mod screen;

#[macroquad::main("zborro-piano-trainer")]
async fn main() {
    if env::args().len() != 2 {
        panic!("ERROR : wrong number of arguments! Expected file as arg");
    }

    let path_string = env::args().nth(1).unwrap();
    let path = Path::new(&path_string);

    let song = song::load_song(path);

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
