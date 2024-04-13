extern crate dirs;
extern crate gl;
extern crate image;
extern crate nalgebra_glm as glm;

use std::{fs, cmp, env};
use std::time::*;
use maze_renderer::RenderingAPI;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use ini::Ini;

mod maze_generator;
mod maze_renderer;

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

use maze_generator::{MazeGenerator, SelectedGenerator, Direction};

use crate::maze_renderer::gl_renderer::GLRenderer;
use crate::maze_renderer::vulkan_renderer::VulkanRenderer;
use crate::maze_renderer::{MazeRenderer, UniformData};

                                    //Vertex position   //Texture UV    //Normal vector
static VERTEX_DATA: [f32; 32] =   [ 0.5,  0.5, 0.0,     1.0, 1.0,       0.0, 0.0, 1.0,
                                    0.5, -0.5, 0.0,     1.0, 0.0,       0.0, 0.0, 1.0,
                                    -0.5, -0.5, 0.0,    0.0, 0.0,       0.0, 0.0, 1.0,
                                    -0.5,  0.5, 0.0,    0.0, 1.0,       0.0, 0.0, 1.0];

static VERTEX_INDICES: [u32; 6] = [0, 1, 3, //First triangle
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
    selected_generator: SelectedGenerator,
    rendering_api: RenderingAPI
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

        //Set rendering API
        if argument.contains("-rendering-api=") && argument.len() > 15 {
            let slice = &argument[15..];

            match slice {
                "OpenGL" => config.rendering_api = RenderingAPI::OPENGL,
                _ => config.rendering_api = RenderingAPI::VULKAN
            }
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
        rendering_api: RenderingAPI::VULKAN
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
                .set("Audio", "1")
                .set("RenderingAPI", "Vulkan");

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

            match section.get("RenderingAPI").unwrap() {
                "Vulkan" => program_config.rendering_api = RenderingAPI::VULKAN,
                _ => program_config.rendering_api = RenderingAPI::OPENGL
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
        window_builder = WindowBuilder::new().with_title("glmaze-rs")
                                                .with_fullscreen(Some(Fullscreen::Borderless(None)));   
    }
    else {
        window_builder = WindowBuilder::new().with_title("glmaze-rs")
                                                .with_inner_size(LogicalSize::new(program_config.window_width, program_config.window_height));   
    }                         

    let window;

    let mut maze_renderer = match program_config.rendering_api {
        RenderingAPI::VULKAN => {
            window = window_builder.build(&event_loop).unwrap();
            let vulkan_renderer = VulkanRenderer::new(&window);

            MazeRenderer::new(Box::new(vulkan_renderer))
        },
        _ => {
            let opengl_renderer = GLRenderer::new(window_builder, &event_loop);
            window = opengl_renderer.1;

            MazeRenderer::new(Box::new(opengl_renderer.0))
        }
    };

    program_config.window_width = window.inner_size().width;
    program_config.window_height = window.inner_size().height;

    //Print selected options
    println!("\nSelected options:");
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
    println!("Rendering API: {}", program_config.rendering_api);

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

    let mut maze_textures_paths = Vec::new();
    maze_textures_paths.push(assets_path.join("wall.png").to_str().unwrap().to_string());
    maze_textures_paths.push(assets_path.join("floor.png").to_str().unwrap().to_string());
    maze_textures_paths.push(assets_path.join("ceiling.png").to_str().unwrap().to_string());
    maze_textures_paths.push(assets_path.join("exit.png").to_str().unwrap().to_string());

    maze_renderer.renderer.load_textures(maze_textures_paths);

    match program_config.rendering_api {
        RenderingAPI::VULKAN => {
            maze_renderer.renderer.load_shaders(shaders_path.join("vk").join("vertexshader.spv").to_str().unwrap(), 
                shaders_path.join("vk").join("fragmentshader.spv").to_str().unwrap());
        },
        RenderingAPI::OPENGL => {
            maze_renderer.renderer.load_shaders(shaders_path.join("gl").join("vertexshader.vert").to_str().unwrap(), 
                shaders_path.join("gl").join("fragmentshader.frag").to_str().unwrap());
        }
    }

    maze_renderer.renderer.init_mesh(VERTEX_DATA.to_vec(), VERTEX_INDICES.to_vec());

    //Setup audio
    let mut audio_manager =
		AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

    let step_sound_data = StaticSoundData::from_file(assets_path.join("steps.wav"), StaticSoundSettings::new().loop_region(0.0..)).unwrap();
    let ambience_sound_data = StaticSoundData::from_file(assets_path.join("ambience.ogg"), StaticSoundSettings::new().loop_region(0.0..)).unwrap();

    //Camera setup
    let mut camera_position = glm::vec3(maze_generator.get_start_position().0 as f32, 0.0, maze_generator.get_start_position().1 as f32);
    let mut camera_front = glm::vec3(0.0, 0.0, -1.0);
    let camera_up = glm::vec3(0.0, 1.0, 0.0);

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
                },
                WindowEvent::Resized(new_size) => {
                    program_config.window_width = new_size.width;
                    program_config.window_height = new_size.height;

                    maze_renderer.renderer.resize_viewport(new_size.width, new_size.height);
                },
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
                let camera_center = camera_position + camera_front;
                let view = glm::look_at(&camera_position, &camera_center, &camera_up);

                //Setup projection matrix
                let projection = match program_config.rendering_api {
                    RenderingAPI::OPENGL => glm::perspective((program_config.window_width as f32)/(program_config.window_height as f32), f32::to_radians(45.0), 0.1, 100.0),
                    RenderingAPI::VULKAN => {
                        let mut projection = glm::perspective_rh_zo((program_config.window_width as f32)/(program_config.window_height as f32), 
                            f32::to_radians(45.0), 0.1, 100.0);
                        projection[5] *= -1.0; //Invert [1][1] component to invert Y on Vulkan

                        projection
                    }
                };

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
                        camera_speed = 80.0 * time_step;
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
                    let camera_direction = glm::vec3(camera_yaw.to_radians().cos() * camera_pitch.to_radians().cos(), 
                        camera_pitch.to_radians().sin(), 
                        camera_yaw.to_radians().sin() * camera_pitch.to_radians().cos());

                    camera_front = glm::normalize(&camera_direction);
                }
                else { 
                    camera_front = glm::vec3(camera_yaw.to_radians().cos(),
                        0.0,
                        camera_yaw.to_radians().sin());
                }

                //End game if player is near to exit
                if check_collision_point_rectangle(camera_position.x, camera_position.z, 
                            maze_generator.get_exit().0 as f32, maze_generator.get_exit().1 as f32) {
                    window_target.exit();
                } 

                //Setup uniforms
                maze_renderer.renderer.update_uniform_data(UniformData {
                    view_matrix: view,
                    projection_matrix: projection,
                    light_position: camera_position,
                    light_color: glm::vec3(1.0, 1.0, 1.0),
                    _padding: Default::default(),
                });

                //Begin rendering
                maze_renderer.renderer.clear_color([0.0, 0.0, 0.0, 1.0]);

                //Maze rendering
                //Only small area around the player needs to be drawn
                //Calculate start and end row and column based on player position
                let start_row = cmp::max(1, camera_position.z as i32 - 10);
                let start_column = cmp::max(1, camera_position.x as i32 - 10);
                let end_row = cmp::min(maze_generator.get_maze_size() as i32 - 1, camera_position.z as i32 + 10);
                let end_column = cmp::min(maze_generator.get_maze_size() as i32 - 1, camera_position.x as i32 + 10);

                for i in start_row..end_row {
                    for j in start_column..end_column {
                        //Don't draw walls around non empty field (they won't be visible)
                        if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + j as usize] {
                            continue;
                        }

                        //Draw walls
                        //Left wall
                        if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + (j - 1) as usize] {                            
                            let mut model = glm::Mat4::identity();
                            model = glm::translate(&model, &glm::vec3((j as f32)*1.0, 0.0, (i as f32)*1.0)); //Move to right position
                            model = glm::translate(&model, &glm::vec3(-0.5, 0.0, 0.0)); //Move left a bit
                            model = glm::rotate(&model, f32::to_radians(-90.0), &glm::vec3(0.0, 1.0, 0.0)); //Rotate by 90 degrees around Y

                            maze_renderer.renderer.draw(model, 0);
                        }

                        //Right wall
                        if maze_generator.get_maze_array()[i as usize * maze_generator.get_maze_size() + (j + 1) as usize] {
                            let mut model = glm::Mat4::identity();
                            model = glm::translate(&model, &glm::vec3((j as f32)*1.0, 0.0, (i as f32)*1.0)); //Move to right position
                            model = glm::translate(&model, &glm::vec3(0.5, 0.0, 0.0)); //Move right a bit
                            model = glm::rotate(&model, f32::to_radians(90.0), &glm::vec3(0.0, 1.0, 0.0)); //Rotate by 90 degrees around Y

                            maze_renderer.renderer.draw(model, 0);
                        }

                        //Front wall
                        if maze_generator.get_maze_array()[(i - 1) as usize * maze_generator.get_maze_size() + j as usize] {
                            let mut model = glm::Mat4::identity();
                            model = glm::translate(&model, &glm::vec3((j as f32)*1.0, 0.0, (i as f32)*1.0)); //Move to right position
                            model = glm::translate(&model, &glm::vec3(0.0, 0.0, -0.5)); //Move front a bit
                            model = glm::rotate(&model, f32::to_radians(180.0), &glm::vec3(0.0, 1.0, 0.0));
                
                            maze_renderer.renderer.draw(model, 0);
                        }

                        //Back wall
                        if maze_generator.get_maze_array()[(i + 1) as usize * maze_generator.get_maze_size() + j as usize] {
                            let mut model = glm::Mat4::identity();
                            model = glm::translate(&model, &glm::vec3((j as f32)*1.0, 0.0, (i as f32)*1.0)); //Move to right position
                            model = glm::translate(&model, &glm::vec3(0.0, 0.0, 0.5)); //Move back a bit
                
                            maze_renderer.renderer.draw(model, 0);
                        }

                        //Floor
                        let mut model = glm::Mat4::identity();
                        model = glm::translate(&model, &glm::vec3((j as f32)*1.0, 0.0, (i as f32)*1.0));
                        model = glm::translate(&model, &glm::vec3(0.0, -0.5, 0.0));
                        model = glm::rotate(&model, f32::to_radians(90.0), &glm::vec3(1.0, 0.0, 0.0));
            
                        maze_renderer.renderer.draw(model, 1);

                        //Ceiling
                        let mut model = glm::Mat4::identity();
                        model = glm::translate(&model, &glm::vec3((j as f32)*1.0, 0.0, (i as f32)*1.0));
                        model = glm::translate(&model, &glm::vec3(0.0, 0.5, 0.0));
                        model = glm::rotate(&model, f32::to_radians(-90.0), &glm::vec3(1.0, 0.0, 0.0));
            
                        maze_renderer.renderer.draw(model, 2);

                        //Draw exit if it's visible
                        if j == maze_generator.get_exit().0 as i32 && i == maze_generator.get_exit().1 as i32 {
                            let mut model = glm::Mat4::identity();
                    
                            match maze_generator.get_end_border() {
                                Direction::Top => {
                                    model = model * glm::translate(&model, &glm::vec3(j as f32, 0.0, (i as f32) - 0.5));
                                    model = model * glm::rotate(&model, f32::to_radians(180.0), &glm::vec3(0.0, 1.0, 0.0));
                                },
                                Direction::Bottom => {
                                    model = model * glm::translate(&model, &glm::vec3(j as f32, 0.0, (i as f32) + 0.5));
                                },
                                Direction::Left => {
                                    model = model * glm::translate(&model, &glm::vec3((j as f32) - 0.5, 0.0, i as f32));
                                    model = model * glm::rotate(&model, f32::to_radians(-90.0), &glm::vec3(0.0, 1.0, 0.0));
                                },
                                Direction::Right => {
                                    model = model * glm::translate(&model, &glm::vec3((j as f32) + 0.5, 0.0, i as f32));
                                    model = model * glm::rotate(&model, f32::to_radians(90.0), &glm::vec3(0.0, 1.0, 0.0));
                                },
                            }

                            maze_renderer.renderer.draw(model, 3);
                        }
                    }
                }

                //Finish rendering
                maze_renderer.renderer.render();

                window.request_redraw();
            },
            Event::LoopExiting => {
                maze_renderer.renderer.cleanup();
            }
            _ => (),
        }
    }).unwrap();

}
