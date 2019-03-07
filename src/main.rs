use std::collections::HashMap;
use std::default::Default;
use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use chrono::*;
use fps_counter::FPSCounter;
use glium::Program;
use glium::backend::Facade;
use glium::uniforms::{AsUniformValue, Uniforms, UniformValue};
use midgar::{KeyCode, Midgar, MouseButton, Surface};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use same_file::is_same_file;

const SCREEN_SIZE: (u32, u32) = (640, 480);

#[derive(Clone, Copy)]
struct Vertex {
    vertex: [f32; 2],
}
glium::implement_vertex!(Vertex, vertex);

//struct UniformValues {
//    uniforms: HashMap<String, Box<AsUniformValue>>,
//}
//
//impl UniformValues {
//    fn new(program: &Program) -> Self {
//        let mut uniforms = HashMap::new();
//        for (name, uniform) in program.uniforms() {
//            // Check uniform type and get a default value for it.
//            let value = match uniform.ty {
//                Float => 0.0f32,
//                FloatVec2 => (0.0f32, 0.0),
//                FloatVec3 => (0.0f32, 0.0, 0.0),
//                FloatVec4 => (0.0f32, 0.0, 0.0, 0.0),
//                Double => 0.0f64,
//                DoubleVec2 => (0.0f64, 0.0),
//                DoubleVec3 => (0.0f64, 0.0, 0.0),
//                DoubleVec4 => (0.0f64, 0.0, 0.0, 0.0),
//                Int => 0i32,
//                IntVec2 => (0i32, 0),
//                IntVec3 => (0i32, 0, 0),
//                IntVec4 => (0i32, 0, 0, 0),
//                UnsignedInt => 0u32,
//                UnsignedIntVec2 => (0u32, 0),
//                UnsignedIntVec3 => (0u32, 0, 0),
//                UnsignedIntVec4 => (0u32, 0, 0, 0),
//                Int64 => 0i64,
//                Int64Vec2 => (0i64, 0),
//                Int64Vec3 => (0i64, 0, 0),
//                Int64Vec4 => (0i64, 0, 0, 0),
//                UnsignedInt64 => 0u64,
//                UnsignedInt64Vec2 => (0u64, 0),
//                UnsignedInt64Vec3 => (0u64, 0, 0),
//                UnsignedInt64Vec4 => (0u64, 0, 0, 0),
//                Bool => false,
//                BoolVec2 => (false, false),
//                BoolVec3 => (false, false, false),
//                BoolVec4 => (false, false, false, false),
//
//                // TODO: Return a result instead of panicking.
//                ty => panic!("Uniforms of type {:?} are unimplemented!", ty),
//            };
//            uniforms.insert(name.clone(), value);
//        }
//        UniformValues {
//            uniforms: uniforms,
//        }
//    }
//}
//
//impl Uniforms for UniformValues {
//    fn visit_values<'a, F>(&'a self, mut output: F)
//        where F: FnMut(&str, UniformValue<'a>) {
//        for (name, value) in &self.uniforms {
//            output(name, value.as_uniform_value());
//        }
//    }
//}

struct ShadertoyUniformValues {
    // (vec3) iResolution, image, The viewport resolution (z is pixel aspect ratio, usually 1.0)
    resolution: [f32; 3],
    // (float) iGlobalTime, image/sound, Current time in seconds
    global_time: f32,
    // (float) iTimeDelta, image, Time it takes to render a frame, in seconds
    time_delta: f32,
    // (int) iFrame, image, Current frame
    frame: i32,
    // (float) iFrameRate, image, Number of frames rendered per second
    frame_rate: f32,
    // (float) iChannelTime[4], image, Time for channel (if video or sound), in seconds
    //channel_time: [f32; 4],
    // (vec3) iChannelResolution[4], image/sound, Input texture resolution for each channel
    //channel_resolution: [[f32;3]; 4],
    // (vec4) iMouse, image, xy = current pixel coords (if LMB is down). zw = click pixel
    mouse: [f32; 4],
    // (sampler2D), iChannel{i}, image/sound, Sampler for input textures i
    //channel: Sampler2D,
    // (vec4) iDate, image/sound, Year, month, day, time in seconds in .xyzw
    date: [f32; 4],
    // (float) iSampleRate, image/sound, The sound sample rate (typically 44100)
    //sample_rate: f32,
}

impl ShadertoyUniformValues {
    fn new() -> Self {
        ShadertoyUniformValues {
            resolution: Default::default(),
            global_time: 0.0,
            time_delta: 0.0,
            frame: 0,
            frame_rate: 0.0,
            //channel_time: Default::default(),
            //channel_resolution: Default::default(),
            mouse: Default::default(),
            //channel: Sampler2D,
            date: Default::default(),
            //sample_rate: 0.0,
        }
    }
}

impl Uniforms for ShadertoyUniformValues {
    fn visit_values<'a, F>(&'a self, mut output: F)
        where F: FnMut(&str, UniformValue<'a>) {
            output("iResolution", self.resolution.as_uniform_value());
            output("iGlobalTime", self.global_time.as_uniform_value());
            output("iTimeDelta", self.time_delta.as_uniform_value());
            output("iFrame", self.frame.as_uniform_value());
            output("iFrameRate", self.frame_rate.as_uniform_value());
            //output("iChannelTime", self.channel_time.as_uniform_value());
            //output("iChannelResolution", self.channel_resolution.as_uniform_value());
            output("iMouse", self.mouse.as_uniform_value());
            //output("iChannel", self.channel.as_uniform_value());
            output("iDate", self.date.as_uniform_value());
            //output("iSampleRate", self.sample_rate.as_uniform_value());
    }
}

struct UiData {
    global_time: f32,
    fps: f32,
    play: bool,
    mouse_button_held: bool,
    mouse_position: [f64; 2],
    date: DateTime<Local>,
}

// TODO: Return a Result to report errors compiling the shader.
fn compile_shader<F>(display: &F, vs_path: &str, fs_path: &str, shadertoy: bool) -> Program
    where F: Facade {
    let vertex_source = fs::read_to_string(vs_path)
        .expect("Could not open vertex shader file");
    let mut fragment_source = fs::read_to_string(fs_path)
        .expect("Could not open vertex shader file");

    if shadertoy {
        fragment_source = format!(
            "#version 150 core

            out vec4 color;

            uniform float iGlobalTime;
            uniform vec3 iResolution;
            uniform vec4 iMouse;
            uniform vec4 iDate;

            {}

            void main() {{
                mainImage(color, gl_FragCoord.xy);
            }}", fragment_source);
        // TODO: Use ShadertoyUniformValues instead of UniformValues
    }

    // NOTE: By default, assume shaders output sRGB colors.
    let program_creation_input = glium::program::ProgramCreationInput::SourceCode {
        vertex_shader: &vertex_source,
        fragment_shader: &fragment_source,
        geometry_shader: None,
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        transform_feedback_varyings: None,
        outputs_srgb: true,
        uses_point_size: false,
    };
    glium::Program::new(display, program_creation_input)
        .expect("Could not compile or link shader program")
    //glium::Program::from_source(
    //    midgar.graphics().display(),
    //    &vertex_source,
    //    &fragment_source,
    //    None,
    //).expect("Could not compile or link shader program")
}

struct App {
    vs_path: String,
    fs_path: String,
    watcher: RecommendedWatcher,
    notify_rx: Receiver<DebouncedEvent>,
    program: glium::Program,
    // TODO: Make this Uniforms
    uniform_values: ShadertoyUniformValues,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u8>,
    ui_data: UiData,
    fps_counter: FPSCounter,
}

impl midgar::App for App {
    fn new(midgar: &Midgar) -> Self {
        let args = clap::App::new("shader_sandbox")
            .args_from_usage(
                "-s --shadertoy 'Treat provided shader as Shadertoy would.'
                <shader_file> 'The shader to run.'")
            .get_matches();

        let vertex_file = "src/shaders/simple.vs.glsl";
        let fragment_file = args.value_of("shader_file")
            .expect("Did not get a shader_file");

        let program = compile_shader(midgar.graphics().display(), vertex_file, fragment_file, args.is_present("shadertoy"));

        let mut uniform_values = ShadertoyUniformValues::new();

        // Use notify to watch for changes in the shaders.
        let (notify_tx, notify_rx) = mpsc::channel();
        let mut watcher = notify::watcher(notify_tx, Duration::from_millis(500))
            .expect("Could not create file watcher");
        watcher.watch(vertex_file, RecursiveMode::NonRecursive)
            .expect("Could not watch vertex shader");
        watcher.watch(fragment_file, RecursiveMode::NonRecursive)
            .expect("Could not watch fragment shader");

        let vertex_data = [
            Vertex { vertex: [-1.0, -1.0] },
            Vertex { vertex: [1.0, -1.0] },
            Vertex { vertex: [1.0, 1.0] },
            Vertex { vertex: [-1.0, 1.0] },
        ];
        let indices = [
            0u8, 1, 3,
            1, 2, 3,
        ];
        let vertex_buffer = glium::VertexBuffer::new(midgar.graphics().display(), &vertex_data)
            .expect("Could not create vertex buffer");
        let index_buffer = glium::IndexBuffer::new(midgar.graphics().display(), glium::index::PrimitiveType::TrianglesList, &indices)
            .expect("Could not create index buffer");

        let screen_size = midgar.graphics().screen_size();
        uniform_values.resolution = [screen_size.0 as f32, screen_size.1 as f32, 1.0];
        uniform_values.iterations = 50;

        let ui_data = UiData {
            global_time: 0.0,
            fps: 0.0,
            play: true,
            mouse_button_held: false,
            mouse_position: [0.0, 0.0],
            date: Local::now(),
        };

        println!("Year: {}\nMonth: {}\nDay: {}\nSecond: {}",
                 ui_data.date.year() as f32,
                 ui_data.date.month() as f32,
                 ui_data.date.day() as f32,
                 ui_data.date.num_seconds_from_midnight() as f32);

        App {
            vs_path: vertex_file.into(),
            fs_path: fragment_file.into(),
            watcher,
            notify_rx,
            program,
            uniform_values,
            vertex_buffer,
            index_buffer,
            ui_data,
            fps_counter: FPSCounter::new(),
        }
    }

    fn step(&mut self, midgar: &mut Midgar) {
        if midgar.input().was_key_pressed(KeyCode::Escape) {
            midgar.set_should_exit();
            return;
        }

        if midgar.input().was_key_pressed(KeyCode::Space) {
            self.ui_data.play = !self.ui_data.play;
        }

        self.ui_data.mouse_button_held = midgar.input().is_button_held(MouseButton::Left);

        let (x, y) = midgar.input().mouse_pos();

        if midgar.input().was_button_pressed(MouseButton::Left) {
            self.uniform_values.mouse[2] = x as f32;
            self.uniform_values.mouse[3] = y as f32;
        }

        if midgar.input().is_button_held(MouseButton::Left) {
            self.uniform_values.mouse[0] = x as f32;
            self.uniform_values.mouse[1] = y as f32;
        }

        // Check if shaders changed, if so, recompile them.
        let recompile_shaders = {
            let mut ret = false;
            while let Ok(event) = self.notify_rx.try_recv() {
                println!("Got file event: {:?}", &event);
                match event {
                    DebouncedEvent::NoticeWrite(path) | DebouncedEvent::Write(path) | DebouncedEvent::Create(path) => {
                        if is_same_file(&path, &self.vs_path).unwrap() || is_same_file(&path, &self.fs_path).unwrap() {
                            ret = true;
                        }
                    },
                    DebouncedEvent::Remove(path) => {
                        if is_same_file(&path, &self.vs_path).unwrap() || is_same_file(&path, &self.fs_path).unwrap() {
                            println!("In-use shader \"{}\" removed! Exiting...", path.display());
                            midgar.set_should_exit();
                            return;
                        }
                    },
                    _ => {},
                }
            }
            ret
        };

        if recompile_shaders {
            // TODO: Any way to recompile shaders in the background?
            print!("Recompiling shaders... ");
            self.program = compile_shader(midgar.graphics().display(), &self.vs_path, &self.fs_path, /*args.is_present("shadertoy")*/ true);
            println!("Done!");
        }

        if !self.ui_data.play && !recompile_shaders {
            return;
        }

        let screen_size = midgar.graphics().screen_size();
        self.uniform_values.resolution = [screen_size.0 as f32, screen_size.1 as f32, 1.0];
        self.uniform_values.time_delta = midgar.time().delta_time() as f32;
        //self.ui_data.global_time += self.uniform_values.time_delta;
        self.uniform_values.global_time += self.uniform_values.time_delta;
        self.uniform_values.frame += 1;
        self.uniform_values.frame_rate = self.fps_counter.tick() as f32;
        self.ui_data.date = Local::now();
        self.uniform_values.date = [
            self.ui_data.date.year() as f32,
            self.ui_data.date.month0() as f32,
            self.ui_data.date.day0() as f32,
            self.ui_data.date.num_seconds_from_midnight() as f32,
        ];

        // Render everything!
        {
            let mut target = midgar.graphics().display().draw();

            // TODO: Do we need to clear the screen?
            target.clear_color(0.0, 0.0, 0.0, 1.0);

            // Run the shader.
            target.draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &self.uniform_values,
                &Default::default(),
            ).expect("Could not draw to screen");

            self.ui_data.fps = self.uniform_values.frame_rate;

            // TODO: Draw UI.

            target.finish().expect("target.finish() failed");
        }
    }
}

fn main() {
    let config = midgar::MidgarAppConfig::new()
        .with_title("Shader Sandbox")
        .with_screen_size(SCREEN_SIZE)
        .with_resizable(true);
    let app: midgar::MidgarApp<App> = midgar::MidgarApp::new(config);
    app.run();
}
