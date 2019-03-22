use std::default::Default;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use chrono::*;
use fps_counter::FPSCounter;
use glium::Program;
use glium::backend::Facade;
use glium::uniforms::{AsUniformValue, Uniforms as GliumUniforms, UniformType, UniformValue};
use imgui::*;
use imgui_glium_renderer::Renderer as ImGuiRenderer;
use imgui_sdl2::ImguiSdl2;
use midgar::{Event, KeyCode, Midgar, MouseButton, Surface};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use toml::Value as TomlValue;

const SCREEN_SIZE: (u32, u32) = (1024, 768);

#[derive(Clone, Copy)]
struct Vertex {
    vertex: [f32; 2],
}
glium::implement_vertex!(Vertex, vertex);

#[derive(Clone, Debug)]
enum StormUniform {
    // User set types.
    Float(f32),
    FloatVec2([f32; 2]),
    FloatVec3([f32; 3]),
    FloatVec4([f32; 4]),
    Double(f64),
    DoubleVec2([f64; 2]),
    DoubleVec3([f64; 3]),
    DoubleVec4([f64; 4]),
    Int(i32),
    IntVec2([i32; 2]),
    IntVec3([i32; 3]),
    IntVec4([i32; 4]),
    UnsignedInt(u32),
    UnsignedIntVec2([u32; 2]),
    UnsignedIntVec3([u32; 3]),
    UnsignedIntVec4([u32; 4]),
    Int64(i64),
    Int64Vec2([i64; 2]),
    Int64Vec3([i64; 3]),
    Int64Vec4([i64; 4]),
    UnsignedInt64(u64),
    UnsignedInt64Vec2([u64; 2]),
    UnsignedInt64Vec3([u64; 3]),
    UnsignedInt64Vec4([u64; 4]),
    Bool(bool),
    BoolVec2([bool; 2]),
    BoolVec3([bool; 3]),
    BoolVec4([bool; 4]),

    // Standard types.
    ColorRgb([f32; 3]),
    ColorRgba([f32; 4]),
    Resolution([f32; 2]),
}

impl AsUniformValue for StormUniform {
    fn as_uniform_value(&self) -> UniformValue {
        use StormUniform::*;
        match self {
            Float(v) => v.as_uniform_value(),
            FloatVec2(v) => v.as_uniform_value(),
            FloatVec3(v) => v.as_uniform_value(),
            FloatVec4(v) => v.as_uniform_value(),
            Double(v) => v.as_uniform_value(),
            DoubleVec2(v) => v.as_uniform_value(),
            DoubleVec3(v) => v.as_uniform_value(),
            DoubleVec4(v) => v.as_uniform_value(),
            Int(v) => v.as_uniform_value(),
            IntVec2(v) => v.as_uniform_value(),
            IntVec3(v) => v.as_uniform_value(),
            IntVec4(v) => v.as_uniform_value(),
            UnsignedInt(v) => v.as_uniform_value(),
            UnsignedIntVec2(v) => v.as_uniform_value(),
            UnsignedIntVec3(v) => v.as_uniform_value(),
            UnsignedIntVec4(v) => v.as_uniform_value(),
            Int64(v) => v.as_uniform_value(),
            Int64Vec2(v) => v.as_uniform_value(),
            Int64Vec3(v) => v.as_uniform_value(),
            Int64Vec4(v) => v.as_uniform_value(),
            UnsignedInt64(v) => v.as_uniform_value(),
            UnsignedInt64Vec2(v) => v.as_uniform_value(),
            UnsignedInt64Vec3(v) => v.as_uniform_value(),
            UnsignedInt64Vec4(v) => v.as_uniform_value(),
            Bool(v) => v.as_uniform_value(),
            BoolVec2(v) => v.as_uniform_value(),
            BoolVec3(v) => v.as_uniform_value(),
            BoolVec4(v) => v.as_uniform_value(),
            ColorRgb(v) => v.as_uniform_value(),
            ColorRgba(v) => v.as_uniform_value(),
            Resolution(v) => v.as_uniform_value(),
        }
    }
}

#[derive(Debug)]
struct UniformHolder {
    name: String,
    value: StormUniform,
}

impl UniformHolder {
    fn new(name: String, value: StormUniform) -> Self {
        Self {
            name,
            value,
        }
    }
}

#[derive(Debug)]
struct FreeformUniforms {
    uniforms: Vec<UniformHolder>,
}

impl FreeformUniforms {
    fn new(program: &Program) -> Self {
        let mut uniforms = Vec::new();
        for (name, uniform) in program.uniforms() {
            use StormUniform::*;

            // Check uniform type and set a default value for it.
            let value = match uniform.ty {
                UniformType::Float => Float(0.0f32),
                UniformType::FloatVec2 => FloatVec2([0.0f32; 2]),
                UniformType::FloatVec3 => FloatVec3([0.0f32; 3]),
                UniformType::FloatVec4 => FloatVec4([0.0f32; 4]),
                UniformType::Double => Double(0.0f64),
                UniformType::DoubleVec2 => DoubleVec2([0.0f64; 2]),
                UniformType::DoubleVec3 => DoubleVec3([0.0f64; 3]),
                UniformType::DoubleVec4 => DoubleVec4([0.0f64; 4]),
                UniformType::Int => Int(0i32),
                UniformType::IntVec2 => IntVec2([0i32; 2]),
                UniformType::IntVec3 => IntVec3([0i32; 3]),
                UniformType::IntVec4 => IntVec4([0i32; 4]),
                UniformType::UnsignedInt => UnsignedInt(0u32),
                UniformType::UnsignedIntVec2 => UnsignedIntVec2([0u32; 2]),
                UniformType::UnsignedIntVec3 => UnsignedIntVec3([0u32; 3]),
                UniformType::UnsignedIntVec4 => UnsignedIntVec4([0u32; 4]),
                UniformType::Int64 => Int64(0i64),
                UniformType::Int64Vec2 => Int64Vec2([0i64; 2]),
                UniformType::Int64Vec3 => Int64Vec3([0i64; 3]),
                UniformType::Int64Vec4 => Int64Vec4([0i64; 4]),
                UniformType::UnsignedInt64 => UnsignedInt64(0u64),
                UniformType::UnsignedInt64Vec2 => UnsignedInt64Vec2([0u64; 2]),
                UniformType::UnsignedInt64Vec3 => UnsignedInt64Vec3([0u64; 3]),
                UniformType::UnsignedInt64Vec4 => UnsignedInt64Vec4([0u64; 4]),
                UniformType::Bool => Bool(false),
                UniformType::BoolVec2 => BoolVec2([false; 2]),
                UniformType::BoolVec3 => BoolVec3([false; 3]),
                UniformType::BoolVec4 => BoolVec4([false; 4]),

                // TODO: Return a result instead of panicking.
                ty => panic!("Uniforms of type {:?} are unimplemented!", ty),
            };
            eprintln!("{}: {:?}", name, uniform.ty);
            uniforms.push(UniformHolder::new(name.clone(), value));
        }
        // TODO: Sort uniforms by name?
        Self {
            uniforms,
        }
    }
}

impl GliumUniforms for FreeformUniforms {
    fn visit_values<'uniform, F>(&'uniform self, mut output: F)
        where F: FnMut(&str, UniformValue<'uniform>) {
        for holder in &self.uniforms {
            output(&holder.name, holder.value.as_uniform_value());
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

impl GliumUniforms for ShadertoyUniforms {
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

enum Uniforms {
    Freeform(FreeformUniforms),
    Shadertoy(ShadertoyUniforms),
}

impl GliumUniforms for Uniforms {
    fn visit_values<'uniform, F>(&'uniform self, output: F)
        where F: FnMut(&str, UniformValue<'uniform>) {
            match self {
                Uniforms::Freeform(uniforms) => uniforms.visit_values(output),
                Uniforms::Shadertoy(uniforms) => uniforms.visit_values(output),
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

fn create_program<F, P>(display: &F, vs_path: &P, fs_path: &P, shadertoy: bool) -> (Program, Uniforms)
    where F: Facade, P: AsRef<Path> {
    let vs_src = fs::read_to_string(&vs_path)
        .expect("Could not open vertex shader file");
    let fs_src = fs::read_to_string(&fs_path)
        .expect("Could not open vertex shader file");

    if shadertoy {
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

        let program = compile_shader(display, &vs_src, &fs_src);

        let uniforms = ShadertoyUniforms::new();
        (program, Uniforms::Shadertoy(uniforms))
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
        let program = compile_shader(display, &vs_src, &fs_src);
        let mut uniforms = FreeformUniforms::new(&program);

        if let TomlValue::Table(table) = parsed_toml {
            for (key, value) in &table {
                if let Some(uniform) = uniforms.uniforms.iter_mut().find(|h| &h.name == key) {
                    // TODO: Do stuff!
                    match value {
                        TomlValue::String(s) if s == "color" => {
                            if let StormUniform::FloatVec3(_) = uniform.value {
                                uniform.value = StormUniform::ColorRgb([1.0; 3]);
                            } else if let StormUniform::FloatVec4(_) = uniform.value {
                                uniform.value = StormUniform::ColorRgba([1.0; 4]);
                            } else {
                                // TODO: Print an error!
                            }
                        }
                        TomlValue::String(s) if s == "resolution" => {
                            if let StormUniform::FloatVec2(_) = uniform.value {
                                uniform.value = StormUniform::Resolution([0.0; 2]);
                            } else {
                                // TODO: Print an error!
                            }
                        }
                        TomlValue::Integer(toml_int) => {
                            if let StormUniform::Int(uniform_int) = &mut uniform.value {
                                *uniform_int = *toml_int as i32;
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

        eprintln!("Uniforms:\n{:?}", uniforms);

        (program, Uniforms::Freeform(uniforms))
    }
}

struct AppState {
    vs_path: PathBuf,
    fs_path: PathBuf,
    watcher: RecommendedWatcher,
    notify_rx: Receiver<DebouncedEvent>,

    program: glium::Program,
    uniforms: Uniforms,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u8>,

    ui_data: UiData,
    fps_counter: FPSCounter,

    imgui: ImGui,
    ui_input_handler: ImguiSdl2,
    ui_renderer: ImGuiRenderer,
}

impl midgar::App for AppState {
    fn new(midgar: &Midgar) -> Self {
        let args = clap::App::new("Shade Storm")
            .args_from_usage(
                "-s --shadertoy 'Treat provided shader as Shadertoy would.'
                <shader_file> 'The shader to run.'")
            .get_matches();

        let vs_path = fs::canonicalize("src/shaders/simple.vs.glsl")
            .expect("Could not canonicalize vertex shader path");
        let fs_path = fs::canonicalize(args.value_of("shader_file")
            .expect("Did not get a shader_file"))
            .expect("Could not canonicalize fragment shader path");
        let (program, uniforms) = create_program(midgar.graphics().display(), &vs_path, &fs_path, args.is_present("shadertoy"));

        // Use notify to watch for changes in the shaders.
        let (notify_tx, notify_rx) = mpsc::channel();
        let mut watcher = notify::watcher(notify_tx, Duration::from_millis(500))
            .expect("Could not create file watcher");
        // NOTE: Watching the parent directory since watching a single file doesn't seem to work...
        let vs_watch_path = vs_path.parent()
            .expect("Could not watch vertex shader");
        watcher.watch(vs_watch_path, RecursiveMode::Recursive)
            .expect("Could not watch vertex shader");
        let fs_watch_path = fs_path.parent()
            .expect("Could not watch fragment shader");
        watcher.watch(fs_watch_path, RecursiveMode::Recursive)
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

        let mut imgui = ImGui::init();
        let ui_input_handler = ImguiSdl2::new(&mut imgui);
        let ui_renderer = ImGuiRenderer::init(&mut imgui, midgar.graphics().display())
            .expect("Could not create ImGui Renderer");

        Self {
            vs_path: vs_path.into(),
            fs_path: fs_path.into(),
            watcher,
            notify_rx,
            program,
            uniforms,
            vertex_buffer,
            index_buffer,
            ui_data,
            fps_counter: FPSCounter::new(),

            imgui,
            ui_input_handler,
            ui_renderer,
        }
    }

    fn event(&mut self, event: &Event, midgar: &mut Midgar) {
        // Send event to imgui.
        self.ui_input_handler.handle_event(&mut self.imgui, event);
        if self.ui_input_handler.ignore_event(event) {
            return;
        }

        // imgui didn't handle the event, so we should!
        if let Event::KeyDown { keycode: Some(KeyCode::Escape), ..} = event {
            midgar.set_should_exit();
            return;
        }

        if let Event::KeyDown { keycode: Some(KeyCode::Space), ..} = event {
            self.ui_data.play = !self.ui_data.play;
        }

        // Handle other global events.
        match *event {
            Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            }
            Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
            }
            Event::MouseMotion { x, y, .. } => {
            }
            Event::MouseWheel { y: 0, .. } => {}
            Event::MouseWheel { y, direction, .. } => {
            }
            _ => {}
        }
    }

    fn step(&mut self, midgar: &mut Midgar) {
        self.ui_data.mouse_button_held = midgar.input().is_button_held(MouseButton::Left);

        let (x, y) = midgar.input().mouse_pos();

        if let Uniforms::Shadertoy(uniforms) = &mut self.uniforms {
            if midgar.input().was_button_pressed(MouseButton::Left) {
                uniforms.mouse[2] = x as f32;
                uniforms.mouse[3] = y as f32;
            }

            if midgar.input().is_button_held(MouseButton::Left) {
                uniforms.mouse[0] = x as f32;
                uniforms.mouse[1] = y as f32;
            }
        }

        // Check if shaders changed, if so, recompile them.
        let recompile_shaders = {
            let mut ret = false;
            while let Ok(event) = self.notify_rx.try_recv() {
                eprintln!("Got file event: {:?}", &event);
                match event {
                    DebouncedEvent::Write(path) | DebouncedEvent::Create(path) => {
                        if path == self.vs_path || path == self.fs_path {
                            ret = true;
                        }
                    },
                    DebouncedEvent::Remove(path) => {
                        if path == self.vs_path || path == self.fs_path {
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
            eprint!("Recompiling shaders... ");
            let shadertoy = if let Uniforms::Shadertoy(_) = self.uniforms {
                true
            } else {
                false
            };
            let (program, uniforms) = create_program(midgar.graphics().display(), &self.vs_path, &self.fs_path, shadertoy);
            self.program = program;
            self.uniforms = uniforms;
            eprintln!("Done!");
        }

        if !self.ui_data.play && !recompile_shaders {
            return;
        }

        // Update uniform values.
        self.ui_data.date = Local::now();
        let screen_size = midgar.graphics().screen_size();
        match &mut self.uniforms {
            Uniforms::Freeform(uniforms) => {
                for holder in &mut uniforms.uniforms {
                    match &mut holder.value {
                        StormUniform::Resolution(v) =>
                            *v = [screen_size.0 as f32, screen_size.1 as f32],
                        _ => {}
                    }
                }
            }
            Uniforms::Shadertoy(uniforms) => {
                uniforms.resolution = [screen_size.0 as f32, screen_size.1 as f32, 1.0];
                uniforms.time_delta = midgar.time().delta_time() as f32;
                //self.ui_data.global_time += self.uniforms.time_delta;
                uniforms.time += uniforms.time_delta;
                uniforms.frame += 1;
                uniforms.frame_rate = self.fps_counter.tick() as f32;
                uniforms.date = [
                    self.ui_data.date.year() as f32,
                    self.ui_data.date.month0() as f32,
                    self.ui_data.date.day0() as f32,
                    self.ui_data.date.num_seconds_from_midnight() as f32,
                ];
            }
        }

        // Update UI.
        let imgui = &mut self.imgui;
        let ui = self.ui_input_handler.frame(
            midgar.graphics().display().window(),
            imgui,
            &midgar.input().mouse_state());

        // Show a window with options for the shader.
        // TODO: Can we dock the window to a side?
        let uniforms = &mut self.uniforms;
        ui.window(im_str!("Shader Options"))
            .size((300.0, 100.0), ImGuiCond::FirstUseEver)
            .build(|| {
                if let Uniforms::Freeform(uniforms) = uniforms {
                    for holder in &mut uniforms.uniforms {
                        // Create a widget to modify the uniform.
                        match &mut holder.value {
                            StormUniform::Int(i) => {
                                ui.slider_int(im_str!("{}", &holder.name), i, 0, 500)
                                    .build();
                            }
                            StormUniform::ColorRgb(c) => {
                                ui.color_edit(im_str!("{}", &holder.name), c)
                                    .build();
                            }
                            StormUniform::ColorRgba(c) => {
                                ui.color_edit(im_str!("{}", &holder.name), c)
                                    .build();
                            }
                            _ => {}
                        }
                    }
                }
            });

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
                &self.uniforms,
                &Default::default(),
            ).expect("Could not draw to screen");

            // Draw the UI.
            self.ui_renderer.render(&mut target, ui)
                .expect("Could not render UI");

            // TODO: Move this somewhere earlier?
            if let Uniforms::Shadertoy(uniforms) = &mut self.uniforms {
                self.ui_data.fps = uniforms.frame_rate;
            }

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
    let app: midgar::MidgarApp<AppState> = midgar::MidgarApp::new(config);
    app.run();
}
