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
use glium::uniforms::{AsUniformValue, Uniforms, UniformType, UniformValue};
use midgar::{KeyCode, Midgar, MouseButton, Surface};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use same_file::is_same_file;
use toml::Value as TomlValue;

const SCREEN_SIZE: (u32, u32) = (640, 480);

#[derive(Clone, Copy)]
struct Vertex {
    vertex: [f32; 2],
}
glium::implement_vertex!(Vertex, vertex);

struct UniformHolder {
    value: Box<dyn AsUniformValue>,
    ty: UniformType,
    resolution: bool,
}

impl UniformHolder {
    fn new(value: Box<dyn AsUniformValue>, ty: UniformType) -> Self {
        Self {
            value,
            ty,
            resolution: false,
        }
    }
}

// TODO: Consider making an enum for the uniform values.
struct FreeformUniforms {
    uniforms: HashMap<String, UniformHolder>,
}

impl FreeformUniforms {
    fn new(program: &Program) -> Self {
        let mut uniforms = HashMap::new();
        for (name, uniform) in program.uniforms() {
            use UniformType::*;

            // Check uniform type and set a default value for it.
            let value: Box<dyn AsUniformValue> = match uniform.ty {
                Float => Box::new(0.0f32),
                FloatVec2 => Box::new([0.0f32; 2]),
                FloatVec3 => Box::new([0.0f32; 3]),
                FloatVec4 => Box::new([0.0f32; 4]),
                Double => Box::new(0.0f64),
                DoubleVec2 => Box::new([0.0f64; 2]),
                DoubleVec3 => Box::new([0.0f64; 3]),
                DoubleVec4 => Box::new([0.0f64; 4]),
                Int => Box::new(0i32),
                IntVec2 => Box::new([0i32; 2]),
                IntVec3 => Box::new([0i32; 3]),
                IntVec4 => Box::new([0i32; 4]),
                UnsignedInt => Box::new(0u32),
                UnsignedIntVec2 => Box::new([0u32; 2]),
                UnsignedIntVec3 => Box::new([0u32; 3]),
                UnsignedIntVec4 => Box::new([0u32; 4]),
                Int64 => Box::new(0i64),
                Int64Vec2 => Box::new([0i64; 2]),
                Int64Vec3 => Box::new([0i64; 3]),
                Int64Vec4 => Box::new([0i64; 4]),
                UnsignedInt64 => Box::new(0u64),
                UnsignedInt64Vec2 => Box::new([0u64; 2]),
                UnsignedInt64Vec3 => Box::new([0u64; 3]),
                UnsignedInt64Vec4 => Box::new([0u64; 4]),
                Bool => Box::new(false),
                BoolVec2 => Box::new([false; 2]),
                BoolVec3 => Box::new([false; 3]),
                BoolVec4 => Box::new([false; 4]),

                // TODO: Return a result instead of panicking.
                ty => panic!("Uniforms of type {:?} are unimplemented!", ty),
            };
            eprintln!("{}: {:?}", name, uniform.ty);
            uniforms.insert(name.clone(), UniformHolder::new(value, uniform.ty));
        }
        Self {
            uniforms,
        }
    }
}

impl Uniforms for FreeformUniforms {
    fn visit_values<'uniform, F>(&'uniform self, mut output: F)
        where F: FnMut(&str, UniformValue<'uniform>) {
        for (name, holder) in &self.uniforms {
            output(name, holder.value.as_uniform_value());
        }
    }
}

#[derive(Debug)]
struct ShadertoyUniforms {
    // (vec3) iResolution, image, The viewport resolution (z is pixel aspect ratio, usually 1.0)
    resolution: [f32; 3],
    // (float) iTime, image/sound, Current time in seconds
    time: f32,
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

impl ShadertoyUniforms {
    fn new() -> Self {
        Self {
            resolution: Default::default(),
            time: 0.0,
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

impl Uniforms for ShadertoyUniforms {
    fn visit_values<'uniform, F>(&'uniform self, mut output: F)
        where F: FnMut(&str, UniformValue<'uniform>) {
            output("iResolution", self.resolution.as_uniform_value());
            output("iTime", self.time.as_uniform_value());
            // The deprecated name for time.
            output("iGlobalTime", self.time.as_uniform_value());
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

enum UniformValues {
    Freeform(FreeformUniforms),
    Shadertoy(ShadertoyUniforms),
}

impl Uniforms for UniformValues {
    fn visit_values<'uniform, F>(&'uniform self, output: F)
        where F: FnMut(&str, UniformValue<'uniform>) {
            match self {
                UniformValues::Freeform(uniforms) => uniforms.visit_values(output),
                UniformValues::Shadertoy(uniforms) => uniforms.visit_values(output),
            }
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
fn compile_shader<F>(display: &F, vs_src: &str, fs_src: &str) -> Program
    where F: Facade {
    // NOTE: By default, assume shaders output sRGB colors.
    let program_creation_input = glium::program::ProgramCreationInput::SourceCode {
        vertex_shader: vs_src,
        fragment_shader: fs_src,
        geometry_shader: None,
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        transform_feedback_varyings: None,
        outputs_srgb: true,
        uses_point_size: false,
    };
    match glium::Program::new(display, program_creation_input) {
        Ok(program) => return program,
        Err(e) => panic!(format!("Error: Could not create shader program:\n{}", e)),
    }
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
    uniform_values: UniformValues,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u8>,
    ui_data: UiData,
    fps_counter: FPSCounter,
}

impl midgar::App for App {
    fn new(midgar: &Midgar) -> Self {
        let args = clap::App::new("Shade Storm")
            .args_from_usage(
                "-s --shadertoy 'Treat provided shader as Shadertoy would.'
                <shader_file> 'The shader to run.'")
            .get_matches();

        let vs_path = "src/shaders/simple.vs.glsl";
        let fs_path = args.value_of("shader_file")
            .expect("Did not get a shader_file");
        let vs_src = fs::read_to_string(vs_path)
            .expect("Could not open vertex shader file");
        let fs_src = fs::read_to_string(fs_path)
            .expect("Could not open vertex shader file");

        let (program, uniform_values) = if args.is_present("shadertoy") {
            let fs_src = format!(
                "#version 150 core

                out vec4 color;

                uniform float iTime;
                uniform float iGlobalTime;
                uniform vec3 iResolution;
                uniform vec4 iMouse;
                uniform vec4 iDate;

                {}

                void main() {{
                    mainImage(color, gl_FragCoord.xy);
                }}", fs_src);

            let program = compile_shader(midgar.graphics().display(), &vs_src, &fs_src);

            let screen_size = midgar.graphics().screen_size();
            let uniform_values = ShadertoyUniforms::new();
            (program, UniformValues::Shadertoy(uniform_values))
        } else {
            let mut split_fs_src = fs_src.split("+++\n");
            // Value before the TOML block.
            split_fs_src.next();

            let toml_src = split_fs_src.next()
                .expect("Did not find TOML block");
            let parsed_toml: TomlValue = toml_src.parse()
                .expect("Could not parse TOML block");
            eprintln!("Parsed TOML:\n{:#?}", parsed_toml);

            let fs_src = split_fs_src.next()
                .expect("Did not find GLSL fragment shader source after TOML block");
            let program = compile_shader(midgar.graphics().display(), &vs_src, &fs_src);
            let mut uniform_values = FreeformUniforms::new(&program);

            if let TomlValue::Table(table) = parsed_toml {
                for (key, value) in &table {
                    if let Some(uniform) = uniform_values.uniforms.get_mut(key) {
                        // TODO: Do stuff!
                        match value {
                            TomlValue::String(s) if s == "resolution" => {
                                if uniform.ty == UniformType::FloatVec2 {
                                    uniform.resolution = true;
                                }
                            }
                            TomlValue::Integer(i) => {
                                if uniform.ty == UniformType::Int {
                                }
                            }
                            //TomlValue::Float(f) => {}
                            //TomlValue::Boolean(b) => {}
                            //TomlValue::Array(arr) => {}
                            _ => {}
                        }
                    }
                }
            }

            (program, UniformValues::Freeform(uniform_values))
        };


        // Use notify to watch for changes in the shaders.
        let (notify_tx, notify_rx) = mpsc::channel();
        let mut watcher = notify::watcher(notify_tx, Duration::from_millis(500))
            .expect("Could not create file watcher");
        watcher.watch(vs_path, RecursiveMode::NonRecursive)
            .expect("Could not watch vertex shader");
        watcher.watch(fs_path, RecursiveMode::NonRecursive)
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

        let ui_data = UiData {
            global_time: 0.0,
            fps: 0.0,
            play: true,
            mouse_button_held: false,
            mouse_position: [0.0, 0.0],
            date: Local::now(),
        };

        eprintln!("Year: {}\nMonth: {}\nDay: {}\nSecond: {}",
                  ui_data.date.year() as f32,
                  ui_data.date.month() as f32,
                  ui_data.date.day() as f32,
                  ui_data.date.num_seconds_from_midnight() as f32);

        App {
            vs_path: vs_path.into(),
            fs_path: fs_path.into(),
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

        if let UniformValues::Shadertoy(uniform_values) = &mut self.uniform_values {
            if midgar.input().was_button_pressed(MouseButton::Left) {
                uniform_values.mouse[2] = x as f32;
                uniform_values.mouse[3] = y as f32;
            }

            if midgar.input().is_button_held(MouseButton::Left) {
                uniform_values.mouse[0] = x as f32;
                uniform_values.mouse[1] = y as f32;
            }
        }

        // Check if shaders changed, if so, recompile them.
        let recompile_shaders = {
            let mut ret = false;
            while let Ok(event) = self.notify_rx.try_recv() {
                eprintln!("Got file event: {:?}", &event);
                match event {
                    DebouncedEvent::NoticeWrite(path) | DebouncedEvent::Write(path) | DebouncedEvent::Create(path) => {
                        if is_same_file(&path, &self.vs_path).unwrap() || is_same_file(&path, &self.fs_path).unwrap() {
                            ret = true;
                        }
                    },
                    DebouncedEvent::Remove(path) => {
                        if is_same_file(&path, &self.vs_path).unwrap() || is_same_file(&path, &self.fs_path).unwrap() {
                            eprintln!("In-use shader \"{}\" removed! Exiting...", path.display());
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
            eprint!("Recompiling shaders... ");
            // TODO: Re-enable this!
            //self.program = compile_shader(midgar.graphics().display(), &self.vs_path, &self.fs_path));
            eprintln!("Done!");
        }

        if !self.ui_data.play && !recompile_shaders {
            return;
        }

        self.ui_data.date = Local::now();
        let screen_size = midgar.graphics().screen_size();
        if let UniformValues::Shadertoy(uniform_values) = &mut self.uniform_values {
            uniform_values.resolution = [screen_size.0 as f32, screen_size.1 as f32, 1.0];
            uniform_values.time_delta = midgar.time().delta_time() as f32;
            //self.ui_data.global_time += self.uniform_values.time_delta;
            uniform_values.time += uniform_values.time_delta;
            uniform_values.frame += 1;
            uniform_values.frame_rate = self.fps_counter.tick() as f32;
            uniform_values.date = [
                self.ui_data.date.year() as f32,
                self.ui_data.date.month0() as f32,
                self.ui_data.date.day0() as f32,
                self.ui_data.date.num_seconds_from_midnight() as f32,
            ];
        }

        // Render everything!
        {
            let mut target = midgar.graphics().display().draw();

            // TODO: Allow the shader to set what to clear the screen to.
            target.clear_color(0.0, 0.0, 0.0, 1.0);

            // Run the shader.
            target.draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &self.uniform_values,
                &Default::default(),
            ).expect("Could not draw to screen");

            if let UniformValues::Shadertoy(uniform_values) = &mut self.uniform_values {
                self.ui_data.fps = uniform_values.frame_rate;
            }

            // TODO: Draw UI.

            target.finish()
                .expect("target.finish() failed");
        }
    }
}

fn main() {
    let config = midgar::MidgarAppConfig::new()
        .with_title("Shade Storm")
        .with_screen_size(SCREEN_SIZE)
        .with_resizable(true);
    let app: midgar::MidgarApp<App> = midgar::MidgarApp::new(config);
    app.run();
}
