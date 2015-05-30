extern crate piston;
extern crate piston_window;
#[macro_use]
extern crate gfx;
extern crate sdl2_window;

use std::cell::RefCell;
use std::rc::Rc;

use piston_window::*;
//use piston::event::*;
use piston::window::{ AdvancedWindow, WindowSettings };
use gfx::traits::*;
use sdl2_window::{ Sdl2Window, OpenGL };

const SCREEN_SIZE: [u32; 2] = [640, 480];

gfx_vertex!( Vertex {
    a_Pos@ pos: [f32; 2],
});

fn main() {
    let window = Rc::new(RefCell::new(Sdl2Window::new(
        OpenGL::_3_2,
        WindowSettings::new("piston-example-gfx_cube", SCREEN_SIZE)
        .exit_on_esc(true)
        .samples(4)
    ).capture_cursor(true)));

    let events = PistonWindow::new(window, empty_app());

    let ref mut factory = events.factory.borrow().clone();

    let vertex_data = [
        Vertex { pos: [-1.0, -1.0] },
        Vertex { pos: [1.0, -1.0] },
        Vertex { pos: [1.0, 1.0] },
        Vertex { pos: [-1.0, 1.0] },
    ];
    let mesh = factory.create_mesh(&vertex_data);
    let slice = mesh.to_slice(gfx::PrimitiveType::TriangleFan);

    let program = {
        let vertex = gfx::ShaderSource {
            glsl_150: Some(include_bytes!("simple.vs")),
            .. gfx::ShaderSource::empty()
        };
        let fragment = gfx::ShaderSource {
            glsl_150: Some(include_bytes!("simple.fs")),
            .. gfx::ShaderSource::empty()
        };
        factory.link_program_source(vertex, fragment).unwrap()
    };

    let state = gfx::DrawState::new();
    let data = None;

    for e in events {
        e.draw_3d(|stream| {
            //let args = e.render_args().unwrap();
            stream.clear(
                gfx::ClearData {
                    color: [0.0, 0.0, 0.0, 1.0],
                    depth: 1.0,
                    stencil: 0,
                }
            );
            stream.draw(&(&mesh, slice.clone(), &program, &data, &state)).unwrap();
        });
    }
}
