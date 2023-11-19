/*TODO: Feature parity with C++ version - it can't be worse than C++ :D
Final review
*/

extern crate sdl2;
extern crate gl;
extern crate image;
extern crate cgmath;

use crate::maze_generator::Direction;

use std::ffi::{CStr, c_void};
use std::mem;
use std::ptr;
use std::cmp;
use std::time::*;
use std::env;
use std::path::Path;
use ini::Ini;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use cgmath::Angle;
use cgmath::EuclideanSpace;
use cgmath::InnerSpace;
use cgmath::{Matrix3, Matrix4, Point3, Vector3, Deg};

use gl::types::*;

mod shader_manager;
mod maze_generator;

use sdl2::event::Event;
use sdl2::keyboard::{Scancode, Keycode};
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::video::FullscreenType;
use shader_manager::ShaderManager;
use maze_generator::MazeGenerator;
use maze_generator::SelectedGenerator;

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
    let end_row = cmp::max(start_row + 4, maze_size as i32);
    let end_column = cmp::max(start_column + 4, maze_size as i32);

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
        seed: String::new(),
        selected_generator: SelectedGenerator::RD,
    };

    if args.iter().any(|e| e.contains("-portable")) {
        program_config.set_portable = true;
    }

    if !program_config.set_portable {
        let config_dir = sdl2::filesystem::pref_path("DragonSWDev", "glmaze-rs").expect("Failed to get config dir.");
        let config_path = Path::new(&config_dir).join("glmaze-rs.ini");

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
                .set("Mouse", "1");

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

    let sdl_context = sdl2::init().unwrap();
    let sdl_video_subsystem = sdl_context.video().unwrap();

    sdl_video_subsystem.gl_attr().set_context_profile(sdl2::video::GLProfile::Core);
    sdl_video_subsystem.gl_attr().set_context_version(3, 3);

    let mut sdl_window = sdl_video_subsystem
            .window("glmaze-rs", program_config.window_width, program_config.window_height)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string()).unwrap();

    //If fullscreen is enabled, get current resolution
    if program_config.set_fullscreen {
        match sdl_video_subsystem.current_display_mode(0) {
            Ok(x) => {
                program_config.window_width = x.w as u32;
                program_config.window_height = x.h as u32;
            }
            Err(_) => { panic!("Error getting current display mode.") }
        }

        sdl_context.mouse().show_cursor(false);
        sdl_window.set_fullscreen(FullscreenType::Desktop).unwrap();
    }

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

    let _gl_context = sdl_window.gl_create_context().unwrap();
    let _gl = gl::load_with(|s| sdl_video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

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

    //TODO paths are not correct
    let install_dir = sdl2::filesystem::base_path().expect("Cannot get install dir.");
    let install_path = Path::new(&install_dir);
    let assets_path = install_path.join("assets");

    //Setup window icon
    //Lack of window icon is not critical error so it it couldn't be loaded continue without it
    if let Ok(icon_file) = image::open(assets_path.join("icon.png")) {
        let width = icon_file.width();
        let height = icon_file.height();
        let mut binding = icon_file.into_rgba8().into_raw();
        let icon_surface = Surface::from_data(binding.as_mut(), width, height, width * 4, PixelFormatEnum::ABGR8888).unwrap();

        sdl_window.set_icon(icon_surface);
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

    //Vsync
    sdl_video_subsystem.gl_set_swap_interval(1).unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    if program_config.mouse_enabled {
        sdl_context.mouse().set_relative_mouse_mode(true);
        sdl_context.mouse().capture(true);
    }

    //Setup game values
    let time_start = Instant::now();

    let mut last_frame = time_start.elapsed().as_secs_f32();
    let mut delta_time = last_frame;

    let mut camera_speed = 90.0 * delta_time;
    let mut movement_speed = 2.5 * delta_time;

    //Main loop
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::MouseMotion { xrel, yrel, .. } => {
                    if program_config.mouse_enabled {
                        let offset_x = xrel as f32 * camera_speed;
                        let offset_y = yrel as f32 * camera_speed;

                        camera_yaw += offset_x;
                        camera_pitch -= offset_y;

                        if camera_pitch > 89.0 {
                           camera_pitch = 89.0;
                        } else if camera_pitch < -89.0 {
                           camera_pitch = -89.0
                        }
                    }
                }
                _ => {}
            }
        }

        //Event handling
        if event_pump.keyboard_state().is_scancode_pressed(Scancode::W) {
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
        }

        if event_pump.keyboard_state().is_scancode_pressed(Scancode::S) {
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
        }

        if event_pump.keyboard_state().is_scancode_pressed(Scancode::A) {
            if !program_config.mouse_enabled {
                camera_yaw -= camera_speed;
            }
        }

        if event_pump.keyboard_state().is_scancode_pressed(Scancode::D) {
            if !program_config.mouse_enabled {
                camera_yaw += camera_speed;
            }
        }

        let current_frame = time_start.elapsed().as_secs_f32();
        delta_time = current_frame - last_frame;
        last_frame = current_frame;
        
        if program_config.mouse_enabled {
            camera_speed = 10.0 * delta_time;
        }
        else {
            camera_speed = 2.5 * delta_time;
        }

        movement_speed = 2.5 * delta_time;

        //End game if player is near to exit
        if check_collision_point_rectangle(camera_position.x, camera_position.z, 
                                            maze_generator.get_exit().0 as f32, maze_generator.get_exit().1 as f32) {
            break 'running;
        }        

        //Setup camera front
        if program_config.mouse_enabled {
            let camera_direction: Vector3<f32> = Vector3::new(cgmath::Deg(camera_yaw).cos() * cgmath::Deg(camera_pitch).cos(), 
                                                            cgmath::Deg(camera_pitch).sin(), 
                                                            cgmath::Deg(camera_yaw).sin() * cgmath::Deg(camera_pitch).cos());

            camera_front = camera_direction.normalize();
        }
        else{ 
            camera_front = Vector3::new(cgmath::Rad(camera_yaw).cos(), 
                                    cgmath::Rad(0.0).sin(), 
                                    cgmath::Rad(camera_yaw).sin());
         } 


        //Begin drawing
        let view = Matrix4::look_at_rh(Point3::from_vec(camera_position), Point3::from_vec(camera_position + camera_front), camera_up);

        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            //Bind wall texture
            gl::BindTexture(gl::TEXTURE_2D, maze_textures[0]);
            
            //Setup uniforms in shaders
            main_shader.use_shader();

            main_shader.set_uniform_matrix4fv("view", view);
            main_shader.set_uniform_matrix4fv("projection", projection);

            main_shader.set_uniform_vec3fv("lightColor", cgmath::Vector3::new(1.0, 1.0, 1.0));
            main_shader.set_uniform_vec3fv("lightVector", camera_position);

            gl::BindVertexArray(vertex_array_object);

            //Draw maze
            //Only small area around the player needs to be drawn
            //Calculate start and end row and column based on player position
            let start_row = cmp::max(1, camera_position.z as i32 - 15);
            let start_column = cmp::max(1, camera_position.x as i32 - 15);
            let end_row = cmp::min(maze_generator.get_maze_size() as i32 - 1, camera_position.z as i32 + 15);
            let end_column = cmp::min(maze_generator.get_maze_size() as i32 - 1, camera_position.x as i32 + 15);

            //Draw maze
            for i in start_row..end_row {
                for j in start_column..end_column {
                    //Don't draw walls around non empty field (they won't be visible)
                    if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + j as usize] {
                        continue;
                    }
                    
                    //Wall texture
                    gl::BindTexture(gl::TEXTURE_2D, maze_textures[0]);

                    //Left wall
                    if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + (j - 1) as usize] {
                        let model = {
                            let position = Matrix4::from_translation(Vector3::new(((j*1) as f32) - 0.5, 0.0, (i*1) as f32));
                            let rotation = Matrix3::from_angle_y(Deg(-90.0));

                            position * Matrix4::from(rotation)
                        };
                        
                        main_shader.set_uniform_matrix4fv("model", model);
                        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                    }

                    //Right wall
                    if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + (j + 1) as usize] {
                        let model = {
                            let position = Matrix4::from_translation(Vector3::new(((j*1) as f32) + 0.5, 0.0, (i*1) as f32));
                            let rotation = Matrix3::from_angle_y(Deg(90.0));

                            position * Matrix4::from(rotation)
                        };
                        
                        main_shader.set_uniform_matrix4fv("model", model);
                        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                    }

                    //Front wall
                    if maze_generator.get_maze_array()[(i - 1) as usize * maze_generator.get_maze_size() + j as usize] {
                        let model = {
                            let position = Matrix4::from_translation(Vector3::new(j as f32, 0.0, ((i*1) as f32) - 0.5));
                            let rotation = Matrix3::from_angle_y(Deg(180.0));

                            position * Matrix4::from(rotation)
                        };
                        
                        main_shader.set_uniform_matrix4fv("model", model);
                        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                    }

                    //Back wall
                    if maze_generator.get_maze_array()[(i + 1) as usize * maze_generator.get_maze_size() + j as usize] {
                        let model = {
                            let position = Matrix4::from_translation(Vector3::new(j as f32, 0.0, ((i*1) as f32) + 0.5));

                            position
                        };
                        
                        main_shader.set_uniform_matrix4fv("model", model);
                        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                    }

                    //Draw floor
                    gl::BindTexture(gl::TEXTURE_2D, maze_textures[1]);

                    let model = {
                        let position = Matrix4::from_translation(Vector3::new((j*1) as f32, -0.5, (i*1) as f32));
                        let rotation = Matrix3::from_angle_x(Deg(90.0));

                        position * Matrix4::from(rotation)
                    };
                    
                    main_shader.set_uniform_matrix4fv("model", model);
                        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);

                    //Draw ceiling
                    gl::BindTexture(gl::TEXTURE_2D, maze_textures[2]);

                    let model = {
                        let position = Matrix4::from_translation(Vector3::new((j*1) as f32, 0.5, (i*1) as f32));
                        let rotation = Matrix3::from_angle_x(Deg(-90.0));

                        position * Matrix4::from(rotation)
                    };
                    
                    main_shader.set_uniform_matrix4fv("model", model);
                    gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);

                    //Draw exit if it's visible
                    let model;
                    if j == maze_generator.get_exit().0 as i32 && i == maze_generator.get_exit().1 as i32 {
                        gl::BindTexture(gl::TEXTURE_2D, maze_textures[3]);

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
                        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
                    }
                }
            }
        }

        sdl_window.gl_swap_window();
    }

}
