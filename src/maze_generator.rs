//Common interface, data and methods for maze generators
pub mod generator_dfs;
pub mod generator_rd;

use core::fmt;

use rand::{
    distributions::{Distribution, Standard},
    Rng
};

use rand_seeder::Seeder;
use rand_pcg::Pcg64;

use self::{generator_rd::GeneratorRD, generator_dfs::GeneratorDFS};

pub enum SelectedGenerator {
    DFS,
    RD
}

impl fmt::Display for SelectedGenerator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SelectedGenerator::DFS => write!(f, "DFS (Depth first search)"),
            SelectedGenerator::RD => write!(f, "RD (Recursive division)")
        }
    }
}

//Cover directions in maze (maze is 2d so only 4 directions)
#[derive(Copy, Clone)]
pub enum Direction {
    Top,
    Bottom,
    Left,
    Right,
}

//Implemeting random for Direction enum
impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..=3) { 
            0 => Direction::Top,
            1 => Direction::Bottom,
            2 => Direction::Left,
            _ => Direction::Right,
        }
    }
}

#[derive(Copy, Clone)]
pub struct PointU32(pub u32, pub u32);

//For every generator that implements Generator trait
//All data related to maze is stored here (including array with maze)
//Generators are supposed to return array for this struct
pub struct MazeGenerator {
    generator: SelectedGenerator,
    maze_size: usize,
    start_position: PointU32,
    end_position: PointU32,
    end_border: Direction,
    maze_array: Vec<bool>,
    random_engine: Pcg64
}

impl MazeGenerator {
    pub fn new(maze_generator: SelectedGenerator, size: usize, seed: String) -> Self {
        MazeGenerator { 
            generator: maze_generator,
            maze_size: size, 
            start_position: PointU32(0, 0), 
            end_position: PointU32(0, 0), 
            end_border: Direction::Top, 
            maze_array: Vec::new(),
            random_engine: Seeder::from(seed).make_rng()
        }
    }

    //Generate maze using selected generator and setup start position and exit 
    pub fn generate_maze(&mut self) {
        match self.generator {
            SelectedGenerator::RD => {
                //RD generator needs odd size
                if self.maze_size % 2 == 0 {
                    self.maze_size += 1;
                }

                let mut generator_rd = GeneratorRD::new(self.maze_size, &mut self.random_engine);
                self.maze_array = generator_rd.generate();
            }

            _ => {
                let mut generator_dfs = GeneratorDFS::new(self.maze_size, &mut self.random_engine);
                self.maze_array = generator_dfs.generate();
            }
        }

        self.set_start_position();
        self.set_exit();
    }

    //Set start position
    //Get two random values and check if their coordinates matches empty (false) field in maze
    //If not generate again in loop
    fn set_start_position(&mut self)  {
        let mut x = self.random_engine.gen_range(1..=(self.maze_size - 1));
        let mut y = self.random_engine.gen_range(1..=(self.maze_size - 1));

        while self.maze_array[y * self.maze_size + x] {
            x = self.random_engine.gen_range(1..=(self.maze_size - 1));
            y = self.random_engine.gen_range(1..=(self.maze_size - 1));
        }

        self.start_position = PointU32(x as u32, y as u32);
    }

    //Setup exit for maze
    //Every maze is supposed to have border around actual maze
    //For exit make a hole in that border but only if it's accesible inside maze (not covered by wall)
    fn set_exit(&mut self)  {
        let mut found_exit = false;

        while !found_exit {
            //Get random index and border then check if it can be used as exit hole
            let exit_index = self.random_engine.gen_range(1..=(self.maze_size - 1));
            let exit_wall: Direction = rand::random();

            match exit_wall {
                Direction::Top => {
                    if self.maze_array[1 * self.maze_size + exit_index] == false {
                        self.end_position = PointU32(exit_index as u32, 1);
                        self.end_border = exit_wall;
                        self.maze_array[0 * self.maze_size + exit_index] = false;

                        found_exit = true;
                    }
                }

                Direction::Bottom => {
                    if self.maze_array[(self.maze_size - 2) * self.maze_size + exit_index] == false {
                        self.end_position = PointU32(exit_index as u32, (self.maze_size - 2) as u32);
                        self.end_border = exit_wall;
                        self.maze_array[(self.maze_size - 1) * self.maze_size + exit_index] = false;

                        found_exit = true;
                    }
                }

                Direction::Left => {
                    if self.maze_array[exit_index * self.maze_size + 1] == false {
                        self.end_position = PointU32(1, exit_index as u32);
                        self.end_border = exit_wall;
                        self.maze_array[exit_index * self.maze_size + 0] = false;

                        found_exit = true;
                    }
                }

                Direction::Right => {
                    if self.maze_array[exit_index * self.maze_size + (self.maze_size - 2)] == false {
                        self.end_position = PointU32((self.maze_size - 2) as u32, exit_index as u32);
                        self.end_border = exit_wall;
                        self.maze_array[exit_index * self.maze_size + (self.maze_size - 1)] = false;

                        found_exit = true;
                    }
                }
            }
        }
    }

    pub fn get_start_position(&self) -> PointU32 {
        self.start_position
    }

    pub fn get_exit(&self) -> PointU32 {
        self.end_position
    }

    pub fn get_end_border(&self) -> Direction {
        self.end_border
    }

    pub fn get_maze_array(&self) -> &Vec<bool> {
        &self.maze_array
    }

    pub fn get_maze_size(&self) -> usize {
        self.maze_size
    }
}
