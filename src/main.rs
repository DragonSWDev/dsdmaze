extern crate dirs;
extern crate gl;
extern crate image;
extern crate cgmath;

use std::ffi::{CStr, c_void, CString};
use std::{fs, mem, ptr, cmp, env};
use std::time::*;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use glutin::config::{GlConfig, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, Version, GlProfile, NotCurrentGlContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::GlSurface;
use glutin_winit::{DisplayBuilder, GlWindow};

use ini::Ini;

use cgmath::{Angle, Deg, EuclideanSpace, InnerSpace, Matrix3, Matrix4, Point3, Vector3};

use gl::types::*;

mod shader_manager;
mod maze_generator;

use raw_window_handle::HasRawWindowHandle;

use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, Event, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{Fullscreen, Icon, WindowBuilder};

use kira::{
	manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
	sound::static_sound::{StaticSoundData, StaticSoundSettings, StaticSoundHandle},
	tween::Tween,
};

use shader_manager::ShaderManager;
use maze_generator::{MazeGenerator, SelectedGenerator, Direction};

                                    //Vertex position   //Texture UV    //Normal vector
static VERTEX_DATA: [GLfloat; 32] = [ 0.5,  0.5, 0.0,    1.0, 1.0,       0.0, 0.0, 1.0,
                                    0.5, -0.5, 0.0,     1.0, 0.0,       0.0, 0.0, 1.0,
                                    -0.5, -0.5, 0.0,    0.0, 0.0,       0.0, 0.0, 1.0,
                                    -0.5,  0.5, 0.0,    0.0, 1.0,       0.0, 0.0, 1.0];

static VERTEX_INDICES: [GLint; 6] = [0, 1, 3, //First triangle
                                      1, 2, 3]; //Second triangle

struct ProgramConfig {
    window_width: u32,
    window_height: u32,
    maze_size: usize,
    enable_collisions: bool,
    set_fullscreen: bool,
    set_portable: bool,
    mouse_enabled: bool,
    audio_enabled: bool,
    seed: String,
    selected_generator: SelectedGenerator
}

//Check collision between point and rectangle
//Used for checking collision between player and maze walls
//In wall position there is margin to avoid camera looking through walls
fn check_collision_point_rectangle(point_x: f32, point_y: f32, wall_x: f32, wall_y: f32) -> bool {
    if point_x >= wall_x - 0.7 && point_x <= wall_x + 0.7 &&
        point_y >= wall_y - 0.7 && point_y <= wall_y + 0.7 {
            return true;
        }
        
    false
}

//Check collision between player and map
fn check_collision(player_x: f32, player_z: f32, maze_size: usize, maze_array: &Vec<bool>) -> bool {
    let mut start_row = player_z as i32;
    let mut start_column = player_x as i32;

    //Only small area around player needs to be checked
    start_row -= 2;
    start_column -= 2;

    //Trim start values to 0 if they are negative
    start_row = cmp::max(start_row, 0);
    start_column = cmp::max(start_column, 0);

    //Get end values and trim it to maze size if they are bigger
    let end_row = cmp::min(start_row + 4, maze_size as i32);
    let end_column = cmp::min(start_column + 4, maze_size as i32);

    let mut collision_occured = false;
    
    for i in start_row..end_row {
        for j in start_column..end_column {
            if maze_array[(i as usize) * maze_size + (j as usize)] 
                && check_collision_point_rectangle(player_x, player_z, j as f32, i as f32) {
                    collision_occured = true;
                }
        }

        if collision_occured {
            break;
        }
    }

    collision_occured
}

//Load image from path and setup OpenGL texture with it
unsafe fn setup_gl_texture(texture_id: GLuint, texture_file: &str) {
    let texture = image::open(texture_file).unwrap().into_rgba8();

    gl::BindTexture(gl::TEXTURE_2D, texture_id);

    //Setup wrapping and filtering
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32, texture.width() as i32, texture.height() as i32, 
                    0, gl::RGBA, gl::UNSIGNED_BYTE, texture.into_raw().as_ptr() as *const c_void);

    gl::GenerateMipmap(gl::TEXTURE_2D);
}

//Parse command line arguments and setup program values
//Get default values if arguments were not provided or they were wrong
fn parse_commandline_arguments(arguments: Vec<String>, config: &mut ProgramConfig) {
    for argument in arguments {
        //Window width
        if argument.contains("-width=") && argument.len() > 7 {
            let slice = &argument[7..];

            config.window_width =  match slice.parse::<u32>() {
                Ok(value) => value, 
                Err(_) => 800,
            }
        }

        //Window height
        if argument.contains("-height=") && argument.len() > 8 {
            let slice = &argument[8..];

            config.window_height =  match slice.parse::<u32>() {
                Ok(value) => value, 
                Err(_) => 600,
            }
        }

        //Maze size
        if argument.contains("-size=") && argument.len() > 6 {
            let slice = &argument[6..];

            config.maze_size =  match slice.parse::<usize>() {
                Ok(value) => value, 
                Err(_) => 600,
            }
        }

        //Generator seed
        if argument.contains("-seed=") && argument.len() > 6 {
            let slice = &argument[6..];
            
            config.seed = String::from(slice);
        }

        //Disable collisions (enabled by default)
        if argument.contains("-disable-collisions") {
            config.enable_collisions = false;
        }

        //Enable fullscreen (disabled by default)
        if argument.contains("-fullscreen") {
            config.set_fullscreen = true;
        }

        //Set maze generator
        if argument.contains("-generator=") && argument.len() > 11 {
            let slice = &argument[11..];

            match slice {
                "DFS" => config.selected_generator = SelectedGenerator::DFS,
                _ => config.selected_generator = SelectedGenerator::RD
            }
        }

        //Disable mouse control (enabled by default)
        if argument.contains("-disable-mouse") {
            config.mouse_enabled = false;
        }

        //Disable mouse control (enabled by default)
        if argument.contains("-disable-audio") {
            config.audio_enabled = false;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut program_config = ProgramConfig {
        window_width: 800,
        window_height: 600,
        maze_size: 20,
        enable_collisions: true,
        set_fullscreen: false,
        set_portable: false,
        mouse_enabled: true,
        audio_enabled: true,
        seed: String::new(),
        selected_generator: SelectedGenerator::RD,
    };

    if args.iter().any(|e| e.contains("-portable")) {
        program_config.set_portable = true;
    }

    if !program_config.set_portable {
        let mut config_path = dirs::config_dir().expect("Failed to get config dir.");
        config_path = config_path.join("DragonSWDev");

        if !config_path.exists() {
            fs::create_dir(config_path.clone()).expect("Failed to create config dir.");
        }

        config_path = config_path.join("glmaze-rs");

        if !config_path.exists() {
            fs::create_dir(config_path.clone()).expect("Failed to create config dir.");
        }

        config_path = config_path.join("glmaze-rs.ini");

        //Config file doesn't exist so create it with default values
        if !config_path.exists() {
            let mut conf = Ini::new();

            conf.with_section(None::<String>).set("encoding", "utf-8");

            conf.with_section(Some("Config"))
                .set("Fullscreen", "0")
                .set("Width", "800")
                .set("Height", "600")
                .set("Size", "20")
                .set("Generator", "RD")
                .set("Collisions", "1")
                .set("Mouse", "1")
                .set("Audio", "1");

            conf.write_to_file(config_path).unwrap();
        } else { //Config file exists, try loading 
            let conf = Ini::load_from_file(config_path).unwrap();
            let section = conf.section(Some("Config")).unwrap();

            if section.get("Fullscreen").unwrap() == "1" {
                program_config.set_fullscreen = true;
            }

            program_config.window_width = section.get("Width").unwrap().parse::<u32>().unwrap();
            program_config.window_height = section.get("Height").unwrap().parse::<u32>().unwrap();
            program_config.maze_size = section.get("Size").unwrap().parse::<usize>().unwrap();

            match section.get("Generator").unwrap() {
                "DFS" => program_config.selected_generator = SelectedGenerator::DFS,
                _ => program_config.selected_generator = SelectedGenerator::RD
            }

            if section.get("Collisions").unwrap() == "0" {
                program_config.enable_collisions = false;
            }

            if section.get("Mouse").unwrap() == "0" {
                program_config.mouse_enabled = false;
            }

            if section.get("Audio").unwrap() == "0" {
                program_config.audio_enabled = false;
            }
        }
    } 

    parse_commandline_arguments(args, &mut program_config);

    //Resolutions restrictions (only for window, full screen uses desktop resolution)
    if program_config.window_width < 100 || program_config.window_width > 7680 || program_config.window_height < 100 
        || program_config.window_height > 4320 || program_config.window_width < program_config.window_height {
            program_config.window_width = 800;
            program_config.window_height = 600;
    }

    //Maze size restrictions
    if program_config.maze_size < 10 || program_config.maze_size > 100000 {
        program_config.maze_size = 20;
    }

    let event_loop = EventLoop::new().unwrap();

    let window_builder;

    if program_config.set_fullscreen {
        window_builder = Some(WindowBuilder::new().with_title("glmaze-rs")
                                                .with_fullscreen(Some(Fullscreen::Borderless(None))));   
    }
    else {
        window_builder = Some(WindowBuilder::new().with_title("glmaze-rs")
                                                .with_resizable(false)
                                                .with_inner_size(LogicalSize::new(program_config.window_width, program_config.window_height)));   
    }                                  

    let display_builder = DisplayBuilder::new().with_window_builder(window_builder);

    let (window, gl_config) = display_builder.build(&event_loop, ConfigTemplateBuilder::new(), |configs| {
        configs
            .reduce(|accum, config| {
                if config.num_samples() > accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
            .unwrap()
    }).unwrap();

    let gl_display = gl_config.display();
    let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());
    let window = window.unwrap();
    let attrs = window.build_surface_attributes(Default::default());

    let gl_surface = unsafe {
        gl_display.create_window_surface(&gl_config, &attrs).unwrap()
    };

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 2))))
        .with_profile(GlProfile::Core)
        .build(raw_window_handle);

    let gl_context = unsafe {
        gl_display.create_context(&gl_config, &context_attributes).expect("Failed to create OpenGL context.").make_current(&gl_surface).unwrap()
    };

    program_config.window_width = window.inner_size().width;
    program_config.window_height = window.inner_size().height;

    //Print selected options
    println!("Selected options:");
    print!("Resolution: {}x{} ", program_config.window_width, program_config.window_height);

    if program_config.set_fullscreen {
        println!("fullscreen");
    }
    else {
        println!("windowed");
    }

    println!("Maze size: {}", program_config.maze_size);
    println!("Collisions: {}", program_config.enable_collisions);
    println!("Mouse control: {}", program_config.mouse_enabled);
    println!("Selected generator: {}", program_config.selected_generator);

    //Generate random seed if it wasn't provided
    if program_config.seed.is_empty() {
        program_config.seed = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    }

    //Setup and generate maze
    let mut maze_generator = MazeGenerator::new(program_config.selected_generator, program_config.maze_size, program_config.seed);
    maze_generator.generate_maze();

    gl::load_with(|symbol| {
        let symbol = CString::new(symbol).unwrap();
        gl_display.get_proc_address(symbol.as_c_str()).cast()
    });

    //Print details about OpenGL context
    println!("OpenGL initialized.");

    unsafe {
        let vendor = gl::GetString(gl::VENDOR) as *const i8;
        let vendor = String::from_utf8(CStr::from_ptr(vendor).to_bytes().to_vec()).unwrap();

        let renderer = gl::GetString(gl::RENDERER) as *const i8;
        let renderer = String::from_utf8(CStr::from_ptr(renderer).to_bytes().to_vec()).unwrap();

        let version = gl::GetString(gl::VERSION) as *const i8;
        let version = String::from_utf8(CStr::from_ptr(version).to_bytes().to_vec()).unwrap();

        println!("Vendor: {}", vendor);
        println!("Renderer: {}", renderer);
        println!("Version: {}", version);
    }

    //Enable depth buffer and face culling
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::CULL_FACE);
    }

    let mut install_path = env::current_exe().expect("Failed to get current path.");
    install_path.pop();
    let assets_path = install_path.join("assets");

    //Setup window icon
    //Lack of window icon is not critical error so it should continue even after icon can't be loaded
    if let Ok(icon_file) = image::open(assets_path.join("icon.png")) {
        let (icon_rgba, icon_width, icon_height) = {
            let icon_rgba8 = icon_file.into_rgba8();
            let (width, height) = icon_rgba8.dimensions();
            let rgba = icon_rgba8.into_raw();
            (rgba, width, height)
        };

        let icon = Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap();
        window.set_window_icon(Some(icon));
    }

    let shaders_path = install_path.join("shaders");

    //Setup shaders
    let mut main_shader = ShaderManager::new();
    main_shader.load_shaders(shaders_path.join("vertexshader.vert").to_str().unwrap(), 
                           shaders_path.join("fragmentshader.frag").to_str().unwrap()).unwrap();

    //Setup VAO, VBO and EBO
    let mut vertex_array_object: GLuint = 0;
    let mut vertex_buffer_object: GLuint = 0;
    let mut element_buffer_object: GLuint = 0;

    unsafe {
        //VAO
        gl::GenVertexArrays(1, &mut vertex_array_object);
        gl::BindVertexArray(vertex_array_object);

        //VBO
        gl::GenBuffers(1, &mut vertex_buffer_object);
        gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer_object);
        gl::BufferData(gl::ARRAY_BUFFER, (VERTEX_DATA.len()*mem::size_of::<GLfloat>()) as GLsizeiptr,
                        VERTEX_DATA.as_ptr() as *const gl::types::GLvoid, gl::STATIC_DRAW);
        
        //VBO Position
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<GLfloat>() as i32, ptr::null());

        //VBO Texture UV
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<GLfloat>() as i32, 
                            (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid);

        //VBO Normal vector
        gl::EnableVertexAttribArray(2);
        gl::VertexAttribPointer(2, 3, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<GLfloat>() as i32, 
                            (5 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid);

        //EBO
        gl::GenBuffers(1, &mut element_buffer_object);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, element_buffer_object);
        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, (VERTEX_INDICES.len()*mem::size_of::<GLint>()) as GLsizeiptr,
                        VERTEX_INDICES.as_ptr() as *const gl::types::GLvoid, gl::STATIC_DRAW);
    }

    //Setup textures
    let mut maze_textures: [GLuint; 4] = [0; 4];
    
    unsafe {
        gl::GenTextures(4, maze_textures.as_mut_ptr());

        setup_gl_texture(maze_textures[0], assets_path.join("wall.png").to_str().unwrap());
        setup_gl_texture(maze_textures[1], assets_path.join("floor.png").to_str().unwrap());
        setup_gl_texture(maze_textures[2], assets_path.join("ceiling.png").to_str().unwrap());
        setup_gl_texture(maze_textures[3], assets_path.join("exit.png").to_str().unwrap());
    }

    //Setup audio
    let mut audio_manager =
		AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

    let step_sound_data = StaticSoundData::from_file(assets_path.join("steps.wav"), StaticSoundSettings::new().loop_region(0.0..)).unwrap();
    let ambience_sound_data = StaticSoundData::from_file(assets_path.join("ambience.ogg"), StaticSoundSettings::new().loop_region(0.0..)).unwrap();

    //Setup projection matrix (model and view matrices will be set in main loop)
    let projection = cgmath::perspective(Deg(45 as f32), 
                                                    (program_config.window_width as f32)/(program_config.window_height as f32), 
                                                    0.1, 100.0);

    //Camera setup
    let mut camera_position: Vector3<f32> = Vector3::new(maze_generator.get_start_position().0 as f32, 0.0, maze_generator.get_start_position().1 as f32);
    let mut camera_front: Vector3<f32> = Vector3::new(0.0, 0.0, -1.0);
    let camera_up: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

    let mut camera_yaw = -90.0;
    let mut camera_pitch = 0.0;

    if program_config.mouse_enabled {
        window.set_cursor_visible(false);
        window.set_cursor_grab(winit::window::CursorGrabMode::Locked).unwrap();
    }

    //Setup game values
    let time_start = Instant::now();
    let mut last_frame = time_start.elapsed().as_secs_f32();
    let time_step: f32 = 0.01;
    let mut accumulator: f32 = 0.0;

    let mut camera_speed = 90.0;

    let mut key_table = vec![false; 255].into_boxed_slice();

    let mut step_sound_playing = false;
    let mut step_sound: Option<StaticSoundHandle> = Default::default();

    if program_config.audio_enabled {
        audio_manager.play(ambience_sound_data).unwrap();
    }

    //Main loop
    event_loop.run(move |event, window_target| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                    event: KeyEvent { logical_key: Key::Named(NamedKey::Escape), .. },
                    ..
                } => window_target.exit(),
                WindowEvent::KeyboardInput { event, .. } => {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        key_table[code as usize] = event.state.is_pressed();
                    }
                }
                _ => (),
            },
            Event::DeviceEvent { event, .. } => {
                match event {
                    DeviceEvent::MouseMotion { delta } => {
                        if program_config.mouse_enabled {
                            let offset_x = delta.0 as f32 * camera_speed;
                            let offset_y = delta.1 as f32 * camera_speed;

                            camera_yaw += offset_x;
                            camera_pitch -= offset_y;

                            if camera_pitch > 89.0 {
                                camera_pitch = 89.0;
                            } else if camera_pitch < -89.0 {
                                camera_pitch = -89.0
                            }
                        }
                    },
                    _ => ()
                }
            }
            Event::AboutToWait => {
                let view = Matrix4::look_at_rh(Point3::from_vec(camera_position), Point3::from_vec(camera_position + camera_front), camera_up);

                let current_frame = time_start.elapsed().as_secs_f32();
                let frame_time = f32::max(0.0, current_frame - last_frame);
                last_frame = current_frame;

                accumulator += frame_time;
                accumulator = f32::clamp(accumulator, 0.0, 1.0);

                //Physics loop
                while accumulator >= time_step {
                    if program_config.mouse_enabled {
                        camera_speed = 10.0 * time_step;
                    }
                    else {
                        camera_speed = 2.5 * time_step;
                    }

                    let movement_speed = 1.4 * time_step;

                    //Process input
                    if key_table[KeyCode::KeyW as usize] {
                        let last_position = camera_position;
                        
    
                        camera_position.x += movement_speed * camera_front.x;
    
                        if program_config.enable_collisions && check_collision(camera_position.x, camera_position.z, 
                                                                maze_generator.get_maze_size(), maze_generator.get_maze_array()) {
                            camera_position = last_position;
                        }
    
                        let last_position = camera_position;
    
                        camera_position.z += movement_speed * camera_front.z;
    
                        if program_config.enable_collisions && check_collision(camera_position.x, camera_position.z, 
                                                                maze_generator.get_maze_size(), maze_generator.get_maze_array()) {
                            camera_position = last_position;
                        }

                        if program_config.audio_enabled && !step_sound_playing {
                            step_sound = Some(audio_manager.play(step_sound_data.clone()).unwrap());
                            step_sound_playing = true;
                        }
                    }
    
                    if key_table[KeyCode::KeyS as usize] {
                        let last_position = camera_position;
    
                        camera_position.x -= movement_speed * camera_front.x;
    
                        if program_config.enable_collisions && check_collision(camera_position.x, camera_position.z, 
                                                                maze_generator.get_maze_size(), maze_generator.get_maze_array()) {
                            camera_position = last_position;
                        }
    
                        let last_position = camera_position;
    
                        camera_position.z -= movement_speed * camera_front.z;
    
                        if program_config.enable_collisions && check_collision(camera_position.x, camera_position.z, 
                                                                maze_generator.get_maze_size(), maze_generator.get_maze_array()) {
                            camera_position = last_position;
                        }

                        if program_config.audio_enabled && !step_sound_playing {
                            step_sound = Some(audio_manager.play(step_sound_data.clone()).unwrap());
                            step_sound_playing = true;
                        }
                    }

                    //Player is not moving so stop step sound if it's playing
                    if !key_table[KeyCode::KeyW as usize] && !key_table[KeyCode::KeyS as usize] && step_sound_playing {
                        if let Some(step_sound) = &mut step_sound {
                            step_sound.stop(Tween::default()).unwrap();
                        }

                        step_sound_playing = false;
                    }
    
                    if key_table[KeyCode::KeyA as usize] {
                        if !program_config.mouse_enabled {
                            camera_yaw -= camera_speed;
                        }
                    }
    
                    if key_table[KeyCode::KeyD as usize] {
                        if !program_config.mouse_enabled {
                            camera_yaw += camera_speed;
                        }
                    }

                    accumulator -= time_step;
                }
        

                //Setup camera front
                if program_config.mouse_enabled {
                    let camera_direction: Vector3<f32> = Vector3::new(cgmath::Deg(camera_yaw).cos() * cgmath::Deg(camera_pitch).cos(), 
                                                                    cgmath::Deg(camera_pitch).sin(), 
                                                                    cgmath::Deg(camera_yaw).sin() * cgmath::Deg(camera_pitch).cos());

                    camera_front = camera_direction.normalize();
                }
                else { 
                    camera_front = Vector3::new(cgmath::Rad(camera_yaw).cos(), 
                                                cgmath::Rad(0.0).sin(), 
                                                cgmath::Rad(camera_yaw).sin());
                }

                //End game if player is near to exit
                if check_collision_point_rectangle(camera_position.x, camera_position.z, 
                            maze_generator.get_exit().0 as f32, maze_generator.get_exit().1 as f32) {
                    window_target.exit();
                } 

                //Setup uniforms in shaders
                main_shader.use_shader();

                main_shader.set_uniform_matrix4fv("view", view);
                main_shader.set_uniform_matrix4fv("projection", projection);

                main_shader.set_uniform_vec3fv("lightColor", cgmath::Vector3::new(1.0, 1.0, 1.0));
                main_shader.set_uniform_vec3fv("lightVector", camera_position);

                //Begin rendering
                unsafe {
                    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                    gl::BindTexture(gl::TEXTURE_2D, maze_textures[0]);
                    
                    gl::BindVertexArray(vertex_array_object);
                }

                //Maze rendering
                //Only small area around the player needs to be drawn
                //Calculate start and end row and column based on player position
                let start_row = cmp::max(1, camera_position.z as i32 - 15);
                let start_column = cmp::max(1, camera_position.x as i32 - 15);
                let end_row = cmp::min(maze_generator.get_maze_size() as i32 - 1, camera_position.z as i32 + 15);
                let end_column = cmp::min(maze_generator.get_maze_size() as i32 - 1, camera_position.x as i32 + 15);

                for i in start_row..end_row {
                    for j in start_column..end_column {
                        //Don't draw walls around non empty field (they won't be visible)
                        if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + j as usize] {
                            continue;
                        }

                        //Bind wall texture
                        unsafe {
                            gl::BindTexture(gl::TEXTURE_2D, maze_textures[0]);
                        }

                        //Draw walls
                        //Left wall
                        if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + (j - 1) as usize] {
                            let model = {
                                let position = Matrix4::from_translation(Vector3::new(((j*1) as f32) - 0.5, 0.0, (i*1) as f32));
                                let rotation = Matrix3::from_angle_y(Deg(-90.0));
                                position * Matrix4::from(rotation)
                            };
                
                            main_shader.set_uniform_matrix4fv("model", model);

                            unsafe {
                                gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                            }
                        }

                        //Right wall
                        if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + (j + 1) as usize] {
                            let model = {
                                let position = Matrix4::from_translation(Vector3::new(((j*1) as f32) + 0.5, 0.0, (i*1) as f32));
                                let rotation = Matrix3::from_angle_y(Deg(90.0));
                                position * Matrix4::from(rotation)
                            };
                
                            main_shader.set_uniform_matrix4fv("model", model);

                            unsafe {
                                gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                            }
                        }

                        //Front wall
                        if maze_generator.get_maze_array()[(i - 1) as usize * maze_generator.get_maze_size() + j as usize] {
                            let model = {
                                let position = Matrix4::from_translation(Vector3::new(j as f32, 0.0, ((i*1) as f32) - 0.5));
                                let rotation = Matrix3::from_angle_y(Deg(180.0));
                                position * Matrix4::from(rotation)
                            };
                
                            main_shader.set_uniform_matrix4fv("model", model);

                            unsafe {
                                gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                            }
                        }

                        //Back wall
                        if maze_generator.get_maze_array()[(i + 1) as usize * maze_generator.get_maze_size() + j as usize] {
                            let model = {
                                let position = Matrix4::from_translation(Vector3::new(j as f32, 0.0, ((i*1) as f32) + 0.5));
                                position
                            };
                
                            main_shader.set_uniform_matrix4fv("model", model);

                            unsafe {
                                gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                            }
                        }

                        //Floor
                        unsafe {
                            gl::BindTexture(gl::TEXTURE_2D, maze_textures[1]);
                        }

                        let model = {
                            let position = Matrix4::from_translation(Vector3::new((j*1) as f32, -0.5, (i*1) as f32));
                            let rotation = Matrix3::from_angle_x(Deg(90.0));
                            position * Matrix4::from(rotation)
                        };
            
                        main_shader.set_uniform_matrix4fv("model", model);

                        unsafe {
                            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                        }

                        //Ceiling
                        unsafe {
                            gl::BindTexture(gl::TEXTURE_2D, maze_textures[2]);
                        }

                        let model = {
                            let position = Matrix4::from_translation(Vector3::new((j*1) as f32, 0.5, (i*1) as f32));
                            let rotation = Matrix3::from_angle_x(Deg(-90.0));
                            position * Matrix4::from(rotation)
                        };
                            
                        main_shader.set_uniform_matrix4fv("model", model);

                        unsafe {
                            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                        }

                        //Draw exit if it's visible
                        let model;
                        if j == maze_generator.get_exit().0 as i32 && i == maze_generator.get_exit().1 as i32 {
                            unsafe {
                                gl::BindTexture(gl::TEXTURE_2D, maze_textures[3]);
                            }
                    
                            match maze_generator.get_end_border() {
                                Direction::Top => {
                                    model = {
                                        let position = Matrix4::from_translation(Vector3::new(j as f32, 0.0, (i as f32) - 0.5));
                                        let rotation = Matrix3::from_angle_y(Deg(180.0));
    
                                        position * Matrix4::from(rotation)
                                    };
                                },
                                Direction::Bottom => {
                                    model = {
                                        let position = Matrix4::from_translation(Vector3::new(j as f32, 0.0, (i as f32) + 0.5));
    
                                        position
                                    };
                                },
                                Direction::Left => {
                                    model = {
                                        let position = Matrix4::from_translation(Vector3::new((j as f32) - 0.5, 0.0, i as f32));
                                        let rotation = Matrix3::from_angle_y(Deg(-90.0));
    
                                        position * Matrix4::from(rotation)
                                    };
                                },
                                Direction::Right => {
                                    model = {
                                        let position = Matrix4::from_translation(Vector3::new((j as f32) + 0.5, 0.0, i as f32));
                                        let rotation = Matrix3::from_angle_y(Deg(90.0));
    
                                        position * Matrix4::from(rotation)
                                    };
                                },
                            }

                            main_shader.set_uniform_matrix4fv("model", model);

                            unsafe {
                                gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                            }
                        }
                    }
                }

                //Finish rendering
                gl_surface.swap_buffers(&gl_context).unwrap();
                window.request_redraw();
            },
            _ => (),
        }
    }).unwrap();

}
