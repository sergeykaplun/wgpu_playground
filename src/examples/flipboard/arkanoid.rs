use std::{cell::Cell, mem};

use glm::{Vec2, UVec2, distance};
use ncollide2d::{shape::{ShapeHandle, Plane, Ball, Cuboid}, na::{Vector2, Isometry2, self}, pipeline::{CollisionGroups, GeometricQueryType, CollisionObjectSlabHandle, ContactEvent}, world::CollisionWorld};
use wgpu::{Buffer, Device, util::{DeviceExt, BufferInitDescriptor}, BufferUsages, BindGroupLayoutEntry, BindGroupEntry, Queue};
use winit::event::{WindowEvent, VirtualKeyCode, ElementState};

use super::FlapPad;

type World = CollisionWorld<f32, CollisionObjectData>;

pub(crate) struct Arkanoid {
    output_cells: Vec<f32>,
    gamedata_buffer: Buffer,
    game_state: GameState,
    directions_pressed: [bool; 2],
    
    world: World,
    ball: CollisionObjectSlabHandle,
    gamepad: CollisionObjectSlabHandle,
    blocks: Vec<CollisionObjectSlabHandle>
}

#[derive(PartialEq)]
enum GameState {
    Ready,
    Running,
    Ended,
}

#[derive(Clone)]
enum CollisionObjectData {
    Ball { velocity: Cell<Vector2<f32>> },
    OuterWall,
    Floor,
    Gamepad,
    Block
}

impl Arkanoid {
    const OUTPUT_RESOLUTION: UVec2 = UVec2::new(FlapPad::RESOLUTION[0], FlapPad::RESOLUTION[1]);
    const GAMEDATA_SIZE: usize = (FlapPad::RESOLUTION[0] * FlapPad::RESOLUTION[1] * (mem::size_of::<[f32; 2]>() as u32)) as usize;
    const ASPECT: Vec2 = Vec2::new((FlapPad::RESOLUTION[0] as f32)/(FlapPad::RESOLUTION[1] as f32), 1.0);
    const BLOCK_SIZE: Vec2 = Vec2::new(0.2, 1.0/32.0);
    const GAMEPAD_SIZE: Vec2 = Vec2::new(0.125, 1.0/32.0);
    const FLAP_END: f32 = 1.0 - 1e-3;
    const BALL_RAD: f32 = 0.075;
    const LEFT: usize = 0; const RIGHT: usize = 1;
    
    const INITIAL_BALL_SPEED: f32 = 0.025;
    const GAMEPAD_SPEED: f32 = 0.15;
    const FLAP_SPEED: f32 = 1.;

    pub fn new(device: &Device) -> Self{
        let output_cells = vec![0.0; Arkanoid::GAMEDATA_SIZE];
        //TODO we don't want to have buffer here
        let gamedata_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Game data buffer"),
            contents: bytemuck::cast_slice(&output_cells),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let mut world = CollisionWorld::new(0.02);
        let contacts_query  = GeometricQueryType::Contacts(0.0, 0.0);
        
        Self::add_world_bounds(&mut world, contacts_query);
        let ball_handle = Self::add_ball(&mut world, contacts_query);
        let gamepad_handle = Self::add_gamepad(&mut world, contacts_query);
        let blocks_handles = Self::add_blocks(&mut world, contacts_query);
        
        Self {
            output_cells,
            gamedata_buffer,
            game_state: GameState::Ended,
            directions_pressed: [false, false],
            
            world: world,
            ball: ball_handle,
            gamepad: gamepad_handle,
            blocks: blocks_handles,
        }
    }

    fn add_world_bounds(world: &mut World, contacts_query: GeometricQueryType<f32>) {
        let plane_left   = ShapeHandle::new(Plane::new(Vector2::x_axis()));
        let plane_bottom = ShapeHandle::new(Plane::new(Vector2::y_axis()));
        let plane_right  = ShapeHandle::new(Plane::new(-Vector2::x_axis()));
        let plane_top    = ShapeHandle::new(Plane::new(-Vector2::y_axis()));
        let planes_pos = [
            Isometry2::new(Vector2::new(0.0, 0.0), na::zero()),
            Isometry2::new(Vector2::new(0.0, 0.0), na::zero()),
            Isometry2::new(Vector2::new(1.0, 0.0), na::zero()),
            Isometry2::new(Vector2::new(0.0, 1.0), na::zero())
        ];
        let outer_plane_data       = CollisionObjectData::OuterWall;
        let floor_data       = CollisionObjectData::Floor;

        let mut others_groups = CollisionGroups::new();
        others_groups.set_membership(&[2]);
        others_groups.set_whitelist(&[1]);

        world.add(planes_pos[0], plane_left,   others_groups, contacts_query, outer_plane_data.clone());
        world.add(planes_pos[1], plane_bottom, others_groups, contacts_query, floor_data.clone());
        world.add(planes_pos[2], plane_right,  others_groups, contacts_query, outer_plane_data.clone());
        world.add(planes_pos[3], plane_top,    others_groups, contacts_query, outer_plane_data.clone());
    }
    
    fn add_ball(world: &mut World, contacts_query: GeometricQueryType<f32>) -> CollisionObjectSlabHandle {
        let ball = ShapeHandle::new(Ball::new(Arkanoid::BALL_RAD));
        let pos = Vector2::new(0.5, 0.5);
        let ball_pos = Isometry2::new(pos, na::zero());
        
        let mut ball_groups = CollisionGroups::new();
        ball_groups.set_membership(&[1]);
        
        let ball_data        = CollisionObjectData::Ball { velocity: Cell::new(Vector2::new(0.5, 1.0).normalize() * Arkanoid::INITIAL_BALL_SPEED) };
        world.add(ball_pos, ball, ball_groups, contacts_query, ball_data).0
    }

    fn add_gamepad(world: &mut World, contacts_query: GeometricQueryType<f32>) -> CollisionObjectSlabHandle {
        let game_pad = ShapeHandle::new(Cuboid::new(Vector2::new(Self::GAMEPAD_SIZE.x, Self::GAMEPAD_SIZE.y)));;
        let pos = Vector2::new(0.5, Self::GAMEPAD_SIZE.y);
        let gamepad_pos = Isometry2::new(pos, na::zero());
        
        let mut gamepad_groups = CollisionGroups::new();
        gamepad_groups.set_membership(&[2]);
        gamepad_groups.set_whitelist(&[1]);
        
        let gamepad_data        = CollisionObjectData::Gamepad;
        world.add(gamepad_pos, game_pad, gamepad_groups, contacts_query, gamepad_data).0
    }

    fn add_blocks(world: &mut World, contacts_query: GeometricQueryType<f32>) -> Vec<CollisionObjectSlabHandle> {
        let block = ShapeHandle::new(Cuboid::new(Vector2::new(Self::BLOCK_SIZE.x, Self::BLOCK_SIZE.y)));
        let mut block_groups = CollisionGroups::new();
        block_groups.set_membership(&[2]);
        block_groups.set_whitelist(&[1]);
        let block_data        = CollisionObjectData::Block;

        let mut res: Vec<CollisionObjectSlabHandle> = Vec::new();
        let centers = [0.1, 0.3, 0.5, 0.7, 0.9];
        for x in 0..5 {
            for y in 1..3 {
                let pos = Vector2::new(centers[x], 1.0 - (Self::BLOCK_SIZE.y * (y as f32)));
                res.push(world.add(Isometry2::new(pos, na::zero()), block.clone(), block_groups, contacts_query, block_data.clone()).0);
            }
        }
        res
    }
    
    pub fn get_bind_group(&self, index: u32) -> (BindGroupLayoutEntry, BindGroupEntry) {
        (
            wgpu::BindGroupLayoutEntry {
                binding: index,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupEntry {
                binding: index,
                resource: self.gamedata_buffer.as_entire_binding(),
            }
        )
    }

    pub fn tick(&mut self, delta: f32) {
        if self.game_state == GameState::Running {
            self.physics_step(delta);
        }
        
        match self.game_state {
            GameState::Ended => {
                self.show_text("START GAME", [11, 7]);
            },
            GameState::Ready | GameState::Running => {
                self.update_game_pad(delta);

                let ball_object   = self.world.collision_object(self.ball).unwrap();
                let ball_pos: Vec2 = Vec2::new(ball_object.position().translation.vector[0] * Self::ASPECT.x, ball_object.position().translation.vector[1] * Self::ASPECT.y);

                let gamepad_object   = self.world.collision_object(self.gamepad).unwrap();
                let gamepad_pos: Vec2 = Vec2::new(gamepad_object.position().translation.vector[0] * Self::ASPECT.x, gamepad_object.position().translation.vector[1] * Self::ASPECT.y);

                (0..Self::OUTPUT_RESOLUTION.y).for_each(|y|{
                    (0..Self::OUTPUT_RESOLUTION.x).for_each(|x|{
                        let uv = Vec2::new((x as f32)/(Self::OUTPUT_RESOLUTION.x as f32) * Self::ASPECT[0], (y as f32)/(FlapPad::RESOLUTION[1] as f32) * Self::ASPECT[1]);
                        let index = y * Self::OUTPUT_RESOLUTION.x + x;
                        
                        let control_pad = ((uv.x - gamepad_pos.x).abs() - Self::GAMEPAD_SIZE.x * Self::ASPECT.x).max((uv.y - gamepad_pos.y).abs() - 1.0/32.0);
                        let ball = distance(&ball_pos, &uv) - Self::BALL_RAD;
                        
                        let mut min_block = f32::MAX;
                        for &block in self.blocks.iter() {
                            let cur_block_object   = self.world.collision_object(block).unwrap();
                            let cur_block_pos: Vec2 = Vec2::new(cur_block_object.position().translation.vector[0] * Self::ASPECT.x, cur_block_object.position().translation.vector[1] * Self::ASPECT.y);
                            let cur_block_dist = ((uv.x - cur_block_pos.x * Self::ASPECT.x).abs() - Self::BLOCK_SIZE.x * 0.5 * Self::ASPECT.x).max((uv.y - cur_block_pos.y * Self::ASPECT.y).abs() - Self::BLOCK_SIZE.y * 0.5 * Self::ASPECT.y);
                            min_block = min_block.min(cur_block_dist);
                        }

                        let cell_index = (index * 2 + 1) as usize;
                        self.output_cells[cell_index] = if control_pad.min(ball).min(min_block) < 0.0 { 62.0 } else { 0.0 };
                    })
                });
            },
            _ => ()
        }

        for cur_cell in self.output_cells.chunks_mut(2) {
            if cur_cell[0].fract() >= Self::FLAP_END {
                cur_cell[0] = cur_cell[1];
            } else if cur_cell[0].fract() != 0.0 {
                let mut new_val = cur_cell[0].fract() + (delta * Self::FLAP_SPEED);
                if new_val >= Self::FLAP_END {
                    new_val = new_val.min(Self::FLAP_END);
                }
                cur_cell[0] = cur_cell[0].floor() + new_val;
            } else  if cur_cell[0] != cur_cell[1] {
                cur_cell[0] += (delta * Self::FLAP_SPEED);
            }
        }
    }

    pub fn update(&self, queue: &Queue) {
        queue.write_buffer(&self.gamedata_buffer, 0, bytemuck::cast_slice(&self.output_cells));
    }

    pub fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input,
                ..
            } => {
                match self.game_state {
                    GameState::Ended => {
                        if input.virtual_keycode.unwrap() == VirtualKeyCode::Space &&
                           input.state ==  ElementState::Released {
                            self.clear();
                            self.game_state = GameState::Running;
                        }
                    },
                    GameState::Running => {
                        let index = match input.virtual_keycode.unwrap() {
                            VirtualKeyCode::A | VirtualKeyCode::Left => Some(Self::LEFT),
                            VirtualKeyCode::D | VirtualKeyCode::Right => Some(Self::RIGHT),
                            _ => None
                        };
                        if let Some(ind) = index {
                            self.directions_pressed[ind] = input.state == ElementState::Pressed;
                        }
                    },
                    GameState::Ready => {
                        if input.virtual_keycode.unwrap() == VirtualKeyCode::Space &&
                           input.state ==  ElementState::Released{
                            self.game_state = GameState::Running;
                        }
                    },
                };
            },
            _ => {}
        }
    }

    fn clear(&mut self) {
        for cur_cell in self.output_cells.chunks_mut(2) {
            cur_cell[1] = 0.0;
        }
    }

    fn show_text(&mut self, text: &str, start_coord: [u32; 2]) {
        for (i, c) in text.chars().enumerate() {
            self.put_char(c, [start_coord[0] + i as u32, start_coord[1]]);
        }
    }

    const CHAR_MAP: [char; 64] = [' ', '!', '\"', '#', '$', '%', '&', '\'',
                                  '(', ')', '*', '+', ',', '-', '.', '/',
                                  '0', '1', '2', '3', '4', '5', '6', '7',
                                  '8', '9', ':', ';', '<', '=', '>', '?',
                                  '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G',
                                  'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
                                  'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W',
                                  'X', 'Y', 'Z', '[', '\\', ']', '^', '_'];

    fn put_char(&mut self, c: char, coord: [u32; 2]) {
        let char_index = Self::CHAR_MAP.iter().position(|&r| r == c).unwrap();

        let flap_index = (coord[1] * FlapPad::RESOLUTION[0] + coord[0]) * 2 + 1;
        self.output_cells[flap_index as usize] = char_index as f32;
    }

    fn update_game_pad(&mut self, delta: f32) {
        let gamepad_object   = self.world.collision_object(self.gamepad).unwrap();
        let gamepad_pos = gamepad_object.position();

        let mut speed = 0.0;
        if self.directions_pressed[Self::LEFT] {
            speed -= Self::GAMEPAD_SPEED;
        }
        if self.directions_pressed[Self::RIGHT] {
            speed += Self::GAMEPAD_SPEED;
        }
        self.world.set_position(self.gamepad, Isometry2::new(Vector2::new(gamepad_pos.translation.vector[0] + delta * speed, gamepad_pos.translation.vector[1]), na::zero()));
    }

    fn handle_contact_event(world: &World, event: &ContactEvent<CollisionObjectSlabHandle>) -> Option<CollisionResult>{
        if let &ContactEvent::Started(collider1, collider2) = event {
            let pair = world.contact_pair(collider1, collider2, false).unwrap();
            let collector: Vec<_> = pair.3.contacts().collect();
            
            let colliders = [collider1, collider2];
            for (i, &element) in colliders.iter().enumerate(){
                let collider = world.collision_object(element).unwrap();
                match collider.data() {
                    CollisionObjectData::Ball { ref velocity } => {
                        let normal = collector[0].contact.normal;
                        let d = velocity.get().dot(&normal);
                        velocity.set(velocity.get() - 2.0 * d * *normal);
                        let oponent_index = 1 - i;
                        let other_collider = world.collision_object(colliders[oponent_index]).unwrap();
                        match other_collider.data() {
                            CollisionObjectData::Block => {
                                return Some(CollisionResult::BlockHit { handle: colliders[oponent_index] });
                            },
                            _ => ()
                        };
                    },
                    CollisionObjectData::Floor => return Some(CollisionResult::Lost),
                    _ => ()
                };
            };

            return None
        }
        None
    }

    fn physics_step(&mut self, delta: f32) {
        //let mut ended = false;
        let mut collision_results: Vec<Option<CollisionResult>> = Vec::new();
        {
            for event in self.world.contact_events() {
                collision_results.push(Self::handle_contact_event(&self.world, event));
            }
        }
        for col_res in collision_results {
            match col_res {
                Some(res) => match res {
                    CollisionResult::BlockHit { handle } => {
                        self.world.remove(&[handle]);
                        let index = self.blocks.iter().position(|x| *x == handle).unwrap();
                        self.blocks.remove(index);
                    },
                    CollisionResult::Lost => {
                        self.game_ended();
                        return;
                    },
                },
                None => (),
            };
        }
        
        {
            let ball_object   = self.world.collision_object(self.ball).unwrap();
            match ball_object.data() {
                CollisionObjectData::Ball { ref velocity } => {
                    //let ball_velocity = ball_object.data().velocity.as_ref().unwrap();
                    let ball_pos = ball_object.position();
                    self.world.set_position(self.ball, Isometry2::new(ball_pos.translation.vector + delta * velocity.get(), na::zero()));    
                },
                _ => ()
            }
        }
        self.world.update();
    }

    fn game_ended(&mut self) {
        //self.clear();
        self.show_text("GAME OVER", [5, 15]);
        self.game_state = GameState::Ended;
    }
}

#[derive(Clone)]
enum CollisionResult {
    BlockHit { handle: CollisionObjectSlabHandle },
    Lost,
}