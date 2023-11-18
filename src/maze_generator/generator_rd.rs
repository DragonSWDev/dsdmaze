//Maze generator that uses Recursive Division alghorithm
use crate::maze_generator::*;

use rand::Rng;
use rand::distributions::{Distribution, Standard};

pub struct GeneratorRD<'a> {
    maze_size: usize,
    random_engine: &'a mut Pcg64
}

pub enum Orientation {
    Horizontal,
    Vertical
}

impl Distribution<Orientation> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Orientation {
        match rng.gen_range(0..=1) { 
            0 => Orientation::Horizontal,
            _ => Orientation::Vertical
        }
    }
}

impl GeneratorRD<'_> {
    pub fn new(maze_size: usize, random_engine: &mut Pcg64) -> GeneratorRD<'_> {
        GeneratorRD {
            maze_size: maze_size,
            random_engine: random_engine
        }
    }

    pub fn generate(&mut self) -> Vec<bool> {
        //Init array (completely empty)
        let mut maze_array = vec![false; self.maze_size * self.maze_size];

        //Make border
        for n in 0..self.maze_size {
            maze_array[n * self.maze_size] = true;
            maze_array[n] = true;
            maze_array[n * self.maze_size + (self.maze_size - 1)] = true;
            maze_array[(self.maze_size - 1) * self.maze_size + n] = true;
        }

        //Get count of maze fields in allocated array
        //Maze fields are array fields with odd index
        let maze_fields = (self.maze_size - 1) / 2;

        let orientation: Orientation = self.random_engine.gen();

        self.divide_chamber(0, 0, maze_fields - 1, maze_fields - 1, orientation, &mut maze_array);

        maze_array
    }

    fn divide_chamber(&mut self, start_field_x: usize, start_field_y: usize, end_field_x: usize, end_field_y: usize, orientation: Orientation, maze_array: &mut Vec<bool>) {
        if (end_field_x - start_field_x) < 1 || (end_field_y - start_field_y) < 1 {
            return;
        }

        match orientation {
            Orientation::Horizontal => {
                let wall_field = self.random_engine.gen_range(start_field_y..end_field_y);

                //Get array index of randomly selected maze field
                let mut wall_index = wall_field * 2 + 1;
                wall_index += 1; //Wall will be drawn in position next to the selected field

                for n in (start_field_x * 2 + 1)..=(end_field_x * 2 + 2) { //Draw horizontal wall
                    maze_array[wall_index * self.maze_size + n] = true;
                }

                let passage_field = self.random_engine.gen_range(start_field_x..=end_field_x); //Select maze field where passage will be placed

                maze_array[wall_index * self.maze_size + (passage_field * 2 + 1)] = false; //Put passage on wall

                //There are two chambers divided by horizontal wall
                let first_chamber_orientation = self.get_orientation(start_field_x, start_field_y, end_field_x, wall_field);
                let second_chamber_orientation = self.get_orientation(start_field_x, wall_field + 1, end_field_x, end_field_y);

                //Divide created chambers
                self.divide_chamber(start_field_x, start_field_y, end_field_x, wall_field, first_chamber_orientation, maze_array);
                self.divide_chamber(start_field_x, wall_field + 1, end_field_x, end_field_y, second_chamber_orientation, maze_array);      
            }

            _ => {
                let wall_field = self.random_engine.gen_range(start_field_x..end_field_x);

                //Same as before but vertically
                let mut wall_index = wall_field * 2 + 1;
                wall_index += 1;

                for n in (start_field_y * 2 + 1)..=(end_field_y * 2 + 2) { 
                    maze_array[n * self.maze_size + wall_index] = true;
                }

                let passage_field = self.random_engine.gen_range(start_field_y..=end_field_y); 

                maze_array[(passage_field * 2 + 1) * self.maze_size + wall_index] = false;

                let first_chamber_orientation = self.get_orientation(start_field_x, start_field_y, wall_field, end_field_y);
                let second_chamber_orientation = self.get_orientation(wall_field + 1, start_field_y, end_field_x, end_field_y); 

                self.divide_chamber(start_field_x, start_field_y, wall_field, end_field_y, first_chamber_orientation, maze_array);
                self.divide_chamber(wall_field + 1, start_field_y, end_field_x, end_field_y, second_chamber_orientation, maze_array); 
            }
        }
    }

    fn get_orientation(&mut self, start_field_x: usize, start_field_y: usize, end_field_x: usize, end_field_y: usize) -> Orientation {
        let chamber_width = end_field_x - start_field_x;
        let chamber_height = end_field_y - start_field_y;

        if chamber_width > chamber_height
        {
            return Orientation::Vertical;
        } 
    
        if chamber_width < chamber_height
        {
            return Orientation::Horizontal;
        }

        let orientation: Orientation = self.random_engine.gen();

        orientation
    }
}
