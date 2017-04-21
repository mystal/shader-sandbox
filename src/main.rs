extern crate chrono;
extern crate clap;
//extern crate conrod;
extern crate fps_counter;
#[macro_use]
extern crate glium;
extern crate midgar;

use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use chrono::*;
//use conrod::{
//    Colorable,
//    Label,
//    Positionable,
//    Theme,
//    Ui,
//    Widget,
//    WidgetId,
//};
//use conrod::color::white;
use fps_counter::FPSCounter;
use glium::Program;
use glium::uniforms::{AsUniformValue, Uniforms, UniformValue};
use midgar::{KeyCode, Midgar, Mouse, Surface};

const SCREEN_SIZE: (u32, u32) = (640, 480);

#[derive(Clone, Copy)]
struct Vertex {
    vertex: [f32; 2],
}
implement_vertex!(Vertex, vertex);

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

// TODO: Implement ShadertoyUniformValues
struct ShadertoyUniformValues {
    resolution: [f32; 3], // (vec3) iResolution, image, The viewport resolution (z is pixel aspect ratio, usually 1.0)
    global_time: f32, // (float) iGlobalTime, image/sound, Current time in seconds
    time_delta: f32, // (float) iTimeDelta, image, Time it takes to render a frame, in seconds
    frame: i32, // (int) iFrame, image, Current frame
    frame_rate: f32, // (float) iFrameRate, image, Number of frames rendered per second
    //channel_time: [f32; 4], // (float) iChannelTime[4], image, Time for channel (if video or sound), in seconds
    //channel_resolution: [[f32;3]; 4], // (vec3) iChannelResolution[4], image/sound, Input texture resolution for each channel
    mouse: [f32; 4], // (vec4) iMouse, image, xy = current pixel coords (if LMB is down). zw = click pixel
    //channel: Sampler2D, // (sampler2D), iChannel{i}, image/sound, Sampler for input textures i
    date: [f32; 4], // (vec4) iDate, image/sound, Year, month, day, time in seconds in .xyzw
    //sample_rate: f32, // (float) iSampleRate, image/sound, The sound sample rate (typically 44100)
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

//const FPS: WidgetId = 0;
//const TIMER: WidgetId = 1;

struct UiData {
    global_time: f32,
    fps: f32,
    play: bool,
    mouse_button_held: bool,
    mouse_position: [f64; 2],
    date: DateTime<Local>,
}

struct App {
    program: glium::Program,
    // TODO: Make this Uniforms
    uniform_values: ShadertoyUniformValues,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u8>,
    ui_data: UiData,
    fps_counter: FPSCounter,
}

impl midgar::App for App {
    fn create(midgar: &Midgar) -> Self {
        let args = clap::App::new("shader_sandbox")
            .args_from_usage(
                "-s --shadertoy 'Treat provided shader as Shadertoy would.'
                <shader_file> 'The shader to run.'")
            .get_matches();

        // TODO: Use notify to watch for changes in the shaders.

        let vertex_file = "src/shaders/simple.vs.glsl";
        let fragment_file = args.value_of("shader_file")
            .expect("Did not get a shader_file");

        let mut vertex_source = String::new();
        let mut fragment_source = String::new();

        File::open(vertex_file).expect("Could not open vertex shader file")
            .read_to_string(&mut vertex_source);
        File::open(fragment_file).expect("Could not open vertex shader file")
            .read_to_string(&mut fragment_source);

        if /*args.is_present("shadertoy")*/ true {
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
        let program = glium::Program::new(midgar.graphics().display(), program_creation_input)
            .expect("Could not compile or link shader program");
        //let program = glium::Program::from_source(
        //    midgar.graphics().display(),
        //    &vertex_source,
        //    &fragment_source,
        //    None,
        //).expect("Could not compile or link shader program");
        let mut uniform_values = ShadertoyUniformValues::new();

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

        uniform_values.resolution = [SCREEN_SIZE.0 as f32, SCREEN_SIZE.1 as f32, 1.0];

        let mut ui_data = UiData {
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
            program: program,
            uniform_values: uniform_values,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            ui_data: ui_data,
            fps_counter: FPSCounter::new(),
        }
    }

    fn step(&mut self, midgar: &mut Midgar) {
        if midgar.input().was_key_pressed(&KeyCode::Escape) {
            midgar.set_should_exit();
            return;
        }

        if midgar.input().was_key_pressed(&KeyCode::Space) {
            self.ui_data.play = !self.ui_data.play;
        }

        self.ui_data.mouse_button_held = midgar.input().is_button_held(&Mouse::Left);

        let (x, y) = midgar.input().mouse_pos();

        if midgar.input().was_button_pressed(&Mouse::Left) {
            self.uniform_values.mouse[2] = x as f32;
            self.uniform_values.mouse[3] = y as f32;
        }

        if midgar.input().is_button_held(&Mouse::Left) {
            self.uniform_values.mouse[0] = x as f32;
            self.uniform_values.mouse[1] = y as f32;
        }

        if !self.ui_data.play {
            return;
        }

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
        // TODO: Update resolution.

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

            // TODO: Draw the UI.
            self.ui_data.fps = self.uniform_values.frame_rate;
            //Label::new(&format!("{}", ui_data.fps))
            //    .xy(180.0, 180.0)
            //    .font_size(32)
            //    .color(white())
            //    .set(FPS, &mut ui);
            //ui.draw(context, g);
            //Label::new(&format!("{}", ui_data.global_time as i32))
            //    .xy(-180.0, 180.0)
            //    .font_size(32)
            //    .color(white())
            //    .set(TIMER, &mut ui);
            //ui.draw(context, g);
            target.finish().expect("target.finish() failed");
        }
    }
}

fn main() {
    let config = midgar::MidgarAppConfig::new()
        .with_title("Shader Sandbox")
        .with_screen_size(SCREEN_SIZE);
    let app: midgar::MidgarApp<App> = midgar::MidgarApp::new(config);
    app.run();
}
