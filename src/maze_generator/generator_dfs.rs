//Maze generator that uses Depth First Search alghorithm
use crate::maze_generator::*;

use rand::{
    Rng,
    seq::SliceRandom,
};

pub struct GeneratorDFS<'a> {
    maze_size: usize,
    random_engine: &'a mut Pcg64
}

impl GeneratorDFS<'_> {
    pub fn new(maze_size: usize, random_engine: &mut Pcg64) -> GeneratorDFS<'_> {
        GeneratorDFS {
            maze_size: maze_size,
            random_engine: random_engine
        }
    }

    pub fn generate(&mut self) -> Vec<bool> {
        //Init array (completely filled)
        let mut maze_array = vec![true; self.maze_size * self.maze_size];

        //Rand starting point and direction
        let x = self.random_engine.gen_range(3..=(self.maze_size - 3));
        let y = self.random_engine.gen_range(3..=(self.maze_size - 3));
        let direction: Direction = self.random_engine.gen();
        //Mark point as empty (visited)
        maze_array[x * self.maze_size + y] = false;

        //Go to the selected direction
        match direction {
            Direction::Top => self.add_path(&mut maze_array, x, y - 1),
            Direction::Bottom => self.add_path(&mut maze_array, x, y + 1),
            Direction::Left => self.add_path(&mut maze_array, x - 1, y),
            Direction::Right => self.add_path(&mut maze_array, x + 1, y),
        }

        maze_array
    }

    //Check if neighbours were visited and visit them in random order
    fn add_path(&mut self, maze_array: &mut Vec<bool>, x: usize, y: usize) {
        //Check if we are out of bounds
        if x >= self.maze_size - 1 || x < 1 || y < 1 || y >= self.maze_size - 1 {
            return;
        }

        //We are on empty field so return
        if !maze_array[x * self.maze_size + y] {
            return;
        }

        //Count visited neighbours
        let mut count = 0;

        if !maze_array[(x - 1) * self.maze_size + y ] {
            count = count + 1;
        }

        if !maze_array[(x + 1) * self.maze_size + y] {
            count = count + 1;
        }

        if !maze_array[x * self.maze_size + (y - 1)] {
            count = count + 1;
        }

        if !maze_array[x * self.maze_size + (y + 1)] {
            count = count + 1;
        }

        if count > 1 {
            return;
        }

        //Mark actual point as visited
        maze_array[x * self.maze_size + y] = false;

        //Create and init vector with possible directions and then shuffle it in random order
        let mut directions: Vec<Direction> = Vec::new();
        directions.push(Direction::Top);
        directions.push(Direction::Bottom);
        directions.push(Direction::Left);
        directions.push(Direction::Right);

        directions.shuffle(self.random_engine);

        //Visit every neighbour recursively
        for direction in directions.iter() {
            match direction {
                Direction::Top => self.add_path(maze_array, x, y - 1),
                Direction::Bottom => self.add_path(maze_array, x, y + 1),
                Direction::Left => self.add_path(maze_array, x - 1, y),
                Direction::Right => self.add_path(maze_array, x + 1, y),
            }
        }
    }
}
