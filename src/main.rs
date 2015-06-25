extern crate chrono;
extern crate clap;
extern crate conrod;
extern crate fps_counter;
#[macro_use]
extern crate gfx;
extern crate piston_window;
extern crate sdl2_window;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::Path;

use chrono::*;
use clap::App;
use conrod::{
    Colorable,
    Label,
    Positionable,
    Theme,
    Ui,
    Widget,
    WidgetId,
};
use conrod::color::white;
use fps_counter::FPSCounter;
use gfx::{ ParamStorage, UniformValue };
use gfx::device::Resources;
use gfx::device::shade::ProgramInfo;
use gfx::shade::{ ParameterError, ParameterId, ShaderParam };
use gfx::traits::*;
use piston_window::*;
use sdl2_window::Sdl2Window;

const SCREEN_SIZE: [u32; 2] = [640, 480];

struct GenericShaderParams<R: Resources> {
    // TODO: Replace UniformValue with your own data structure/trait.
    uniforms: HashMap<String, UniformValue>,
    _r: PhantomData<R>,
}

impl<R: Resources> GenericShaderParams<R> {
    fn new(program_info: &ProgramInfo) -> GenericShaderParams<R> {
        let mut uniforms = HashMap::new();
        // TODO: If uniform matches a builtin name, but not tye type, panic!
        for uniform in &program_info.uniforms {
            uniforms.insert(uniform.name.clone(), uniform.clone().into());
        }
        GenericShaderParams {
            uniforms: uniforms,
            _r: PhantomData,
        }
    }
}

impl<R: Resources> ShaderParam for GenericShaderParams<R> {
    type Resources = R;
    type Link = Vec<(String, ParameterId)>;

    fn create_link(_: Option<&Self>, program_info: &ProgramInfo)
        -> Result<Self::Link, ParameterError> {
        let mut link = Vec::with_capacity(program_info.uniforms.len());
        for (id, uniform) in program_info.uniforms.iter().enumerate() {
            link.push((uniform.name.clone(), id as ParameterId));
        }
        Ok(link)
    }

    fn fill_params(&self, link: &Self::Link, storage: &mut ParamStorage<R>) {
        use gfx::shade::Parameter;
        for &(ref name, id) in link {
            self.uniforms[name].put(id, storage);
        }
    }
}

gfx_vertex!( Vertex {
    a_Pos@ pos: [f32; 2],
});

const FPS: WidgetId = 0;
const TIMER: WidgetId = 1;

struct UiData {
    global_time: f32,
    fps: usize,
    play: bool,
    mouse_button_held: bool,
    mouse_position: [f64; 2],
    date: DateTime<Local>,
}

fn main() {
    let args =
        App::new("shader_sandbox")
                 .args_from_usage(
                     "-s --shadertoy 'Treat provided shader as Shadertoy would.'
                     <shader_file> 'The shader to run.'")
                 .get_matches();

    let window: PistonWindow<Sdl2Window> = WindowSettings::new("Shader Sandbox", SCREEN_SIZE)
        .exit_on_esc(true)
        .samples(4)
        .into();
    let ref mut factory = window.factory.borrow().clone();

    let vertex_file = "src/simple.vs";
    let fragment_file = args.value_of("shader_file").unwrap();

    let mut vertex_source = String::new();
    let mut fragment_source = String::new();

    File::open(vertex_file).unwrap().read_to_string(&mut vertex_source);
    File::open(fragment_file).unwrap().read_to_string(&mut fragment_source);

    if args.is_present("shadertoy") {
        fragment_source = format!(
            "uniform float iGlobalTime;
            uniform vec3 iResolution;
            uniform vec4 iMouse;
            uniform vec4 iDate;

            {}

            void main() {{
                mainImage(gl_FragColor, gl_FragCoord.xy);
            }}", fragment_source);
    }

    let program = {
        let vertex = gfx::ShaderSource {
            glsl_150: Some(vertex_source.as_bytes()),
            .. gfx::ShaderSource::empty()
        };
        let fragment = gfx::ShaderSource {
            glsl_150: Some(fragment_source.as_bytes()),
            .. gfx::ShaderSource::empty()
        };
        factory.link_program_source(vertex, fragment).unwrap()
    };

    let vertex_data = [
        Vertex { pos: [-1.0, -1.0] },
        Vertex { pos: [1.0, -1.0] },
        Vertex { pos: [1.0, 1.0] },
        Vertex { pos: [-1.0, 1.0] },
    ];
    let mesh = factory.create_mesh(&vertex_data);
    let slice = mesh.to_slice(gfx::PrimitiveType::TriangleFan);

    let state = gfx::DrawState::new();

    let mut params = GenericShaderParams::new(program.get_info());
    params.uniforms.insert("iResolution".to_string(), UniformValue::F32Vector3(
        [SCREEN_SIZE[0] as f32, SCREEN_SIZE[1] as f32, 0.0]));
    params.uniforms.insert("iGlobalTime".to_string(), UniformValue::F32(0.0));
    params.uniforms.insert("iMouse".to_string(), UniformValue::F32Vector4(
        [0.0, 0.0, 0.0, 0.0]));
    params.uniforms.insert("iDate".to_string(), UniformValue::F32Vector4(
        [0.0, 0.0, 0.0, 0.0]));
    params.uniforms.insert("iterations".to_string(), UniformValue::I32(100));

    let mut ui_data = UiData {
        global_time: 0.0,
        fps: 0,
        play: true,
        mouse_button_held: false,
        mouse_position: [0.0, 0.0],
        date: Local::now(),
    };

    let mut fps_counter = FPSCounter::new();

    let glyph_cache = {
        let font_path = Path::new("assets/VeraMono.ttf");
        Glyphs::new(&font_path, factory.clone()).unwrap()
    };
    let mut ui  = Ui::new(glyph_cache, Theme::default());

    //let draw_ui = |c, g, ui: &mut Ui<GlyphCache<_, _>>, ui_data| {
    //};
    println!("Year: {}\nMonth: {}\nDay: {}\nSecond: {}",
             ui_data.date.year() as f32,
             ui_data.date.month() as f32,
             ui_data.date.day() as f32,
             ui_data.date.num_seconds_from_midnight() as f32);

    for e in window {
        if let Some(button) = e.press_args() {
            match button {
                Button::Keyboard(key) => {
                    match key {
                        keyboard::Key::Space => ui_data.play = !ui_data.play,
                        _ => (),
                    }
                },
                Button::Mouse(button) => {
                    match button {
                        mouse::MouseButton::Left => ui_data.mouse_button_held = true,
                        _ => (),
                    }
                },
            }
        }
        if let Some(button) = e.release_args() {
            match button {
                Button::Keyboard(key) => {
                },
                Button::Mouse(button) => {
                    match button {
                        mouse::MouseButton::Left => ui_data.mouse_button_held = false,
                        _ => (),
                    }
                },
            }
        }
        if let Some(mouse_position) = e.mouse_cursor_args() {
            ui_data.mouse_position = mouse_position;
        }
        if ui_data.mouse_button_held {
            params.uniforms.insert("iMouse".to_string(), UniformValue::F32Vector4(
                [ui_data.mouse_position[0] as f32, ui_data.mouse_position[1] as f32, 0.0, 0.0]));
        }

        if let Some(args) = e.update_args() {
            if ui_data.play {
                ui_data.global_time += args.dt as f32;
                ui_data.date = Local::now();
                params.uniforms.insert("iGlobalTime".to_string(), UniformValue::F32(
                    ui_data.global_time));
                params.uniforms.insert(
                    "iDate".to_string(),
                    UniformValue::F32Vector4([
                        ui_data.date.year() as f32,
                        ui_data.date.month0() as f32,
                        ui_data.date.day0() as f32,
                        ui_data.date.num_seconds_from_midnight() as f32,
                    ]));
            }
        }
        if let Some(_) = e.render_args() {
            let size = e.size();
            params.uniforms.insert("iResolution".to_string(), UniformValue::F32Vector3(
                [size.width as f32, size.height as f32, 0.0]));
            e.draw_3d(|stream| {
                stream.clear(
                    gfx::ClearData {
                        color: [0.0, 0.0, 0.0, 1.0],
                        depth: 1.0,
                        stencil: 0,
                    }
                );
                stream.draw(&(&mesh, slice.clone(), &program, &params, &state)).unwrap();
            });
            ui_data.fps = fps_counter.tick();
            e.draw_2d(|context, g| {
                Label::new(&format!("{}", ui_data.fps))
                    .xy(180.0, 180.0)
                    .font_size(32)
                    .color(white())
                    .set(FPS, &mut ui);
                ui.draw(context, g);
                Label::new(&format!("{}", ui_data.global_time as i32))
                    .xy(-180.0, 180.0)
                    .font_size(32)
                    .color(white())
                    .set(TIMER, &mut ui);
                ui.draw(context, g);
            });
        }
    }
}
