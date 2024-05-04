# dsdmaze
Simple 3D maze game written in Rust with OpenGL and Vulkan rendering support. Project started as [glMaze](https://github.com/DragonSWDev/glMaze) rewrite in Rust for learning purposes, currently it's more advanced than original project which lacks features like Vulkan rendering or audio.

<span style="display:block;text-align:center">![Screenshot](./doc/screenshot.png)

## Manual
dsdmaze expects assets and shaders directories to be placed in same directory as binary. OpenGL renderer expects shaders (with .vert and .frag extensions) in "gl" subdirectory, Vulkan renderer expects compiled SPIR-V shaders (with .spv extensions) in "vk" subdirectory. 

### Configuration options
Configurations is specified by command line arguments or by ini configuration file. Command line options can be specified in any order and count. Command line arguments have higher priority and will override config file values. 

**-width=value** - Window width

**-height=value** - Window height
#### Note: These values are respected only if game works in windowed mode. In fullscreen mode game always set desktop resolution. With custom window size both values (width and height) needs to be specified and height can't be bigger than width. Default size is 800x600.

**-size=value** - Maze size (Min is 10, max is 100000, default 20). 
#### Note: For big mazes (more than 1000) it's better to use RD generator because DFS is pretty slow and generating big mazes will last long time even on fast CPU. Big mazes will also consume more memory. For 100000 size (RD generator) application consumes over 9 GiB of RAM.

**-disable-collisions** - Disable collisions

**-fullscreen** - Run in fullscreen mode

**-generator=value** - Select maze generator: "RD" for recursive division and "DFS" for depth-first search. Default is "RD".

**-seed=value** - Generator seed

**-portable** - Don't try to load or create config file

**-disable-mouse** - Disable mouse control

**-disable-audio** - Disable audio

**-rendering-api=value** - Select rendering API: "OpenGL" for OpenGL 3.3 and "Vulkan" for Vulkan 1.0. Default is Vulkan.

**-disable-vsync** - Disable V-Sync

Configuration file is located in following directories:

#### Linux
~/.config/DragonSWDev/dsdmaze/

#### Windows
%appdata%\DragonSWDev\dsdmaze\

#### macOS
~/Library/Application Support/DragonSWDev/dsdmaze/

### Controls
By default camera is controlled by mouse and W/S keys are used for moving forward/backward. When mouse control is disabled then camera is rotated left and right by A and D keys.

## License
dsdmaze is distributed under the terms of MIT License. Project depends on OpenGL, SDL2 and following Rust crates: winit, glutin, glutin-winit, raw-window-handle, dirs, gl, image, nalgebra-glm, rand, rand_seeder, rand_pcg, rust-ini, kira, ash, ash-window, gpu-allocator. For information about licensing check their respecitve websites. Assets are distributed under different licenses, for details check [license.txt](/assets/license.txt) file in assets directory.  
