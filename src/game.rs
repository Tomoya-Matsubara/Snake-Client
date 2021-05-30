use crate::{GameConfig};
use std::io::{stdout, Stdout, Write, Read, Bytes};
use std::fmt::{Display, Formatter, Result};
use termion::event::{parse_event, Event, Key};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::{async_stdin, clear, color, cursor, AsyncReader};
use serde::{Deserialize, Serialize};

// Char representing a border
const BORDER_CHAR: char = '#';
// Char representing the food items
const FOOD_CHAR: char = 'Ծ';
// Char for snakes body
pub const SNAKE_CHAR: char = 'o';

/// Directions
#[derive(Serialize, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    Unkown,
}
impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let direction = match *self {
            Direction::Up => "UP",
            Direction::Down => "DOWN",
            Direction::Left => "LEFT",
            Direction::Right => "RIGHT",
            Direction::Unkown => "UNKOWN",
        };
        write!(f, "{}", direction)
    }
}

/// One coordinate on the field
#[derive(Deserialize, Clone)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}
/// Game Events
#[derive(Deserialize)]
pub enum GameEvent {
    WaitInLobby,
    Start,
    NewTurn,
}
/// Game states
#[derive(Deserialize, PartialEq)]
pub enum GameState {
    Ready,
    Playing,
    Lost,
}

/// Game structure
pub struct Game {
    pub id: usize,
    // Stdout in "raw" mode
    stdout: RawTerminal<Stdout>,
    // Asynchronous stdin to handle user inputs
    stdin: Bytes<AsyncReader>,
    pub snakes: Vec<Vec<Point>>,
    pub direction: Direction,
    pub food: Point,
    field: Vec<Vec<char>>,
    pub killed: bool,
}
impl Game {
    /// Create an empty game
    pub fn empty() -> Game {
        let stdout = stdout().into_raw_mode().unwrap();
        let stdin = async_stdin().bytes();
        let game = Game {
            id: 0,
            stdout: stdout,
            stdin: stdin,
            direction: Direction::Right,
            field: vec![],
            snakes: vec![],
            food: Point { x: 0, y: 0 },
            killed: false,
        };
        return game;
    }

    /// Initialize the game
    pub fn set_config(&mut self, config: GameConfig) {
        self.id = config.id;
        self.field = init_field(config.width, config.height);
        self.snakes = config.snakes;
        self.food = config.food;
    }

    /// Draw the game's borders
    pub fn draw_field(&mut self) {
        // On écrit dans notre console statique dans l'ordre
        // - on efface tout le contenu
        // - place le curseur au début de la première ligne
        // - la couleur du ForeGround choisie est bleu
        write!(
            self.stdout,
            "{}{}{}",
            clear::All,
            cursor::Goto(1, 1),
            color::Fg(color::Blue)
        )
        .unwrap();
        // On appelle flush() pour forcer les modifications dans
        // stdout
        self.stdout.flush().unwrap();

        // Affichage de l'espace de jeu
        let mut i = 0;
        for line in self.field.iter() {
            for c in line.into_iter() {
                write!(self.stdout, "{}", c).unwrap();
            }
            // Passe à la ligne suivante et replace le curseur en début de ligne
            write!(self.stdout, "{}\n", cursor::Goto(1, (i + 1) as u16)).unwrap();
            i += 1;
        }

        // Remet à jour la couleur utilisé
        write!(self.stdout, "{}", color::Fg(color::Reset)).unwrap();
        self.stdout.flush().unwrap();
    }

    /// Draw the food
    pub fn draw_food(&mut self) {
        // 4 étapes
        // - place le curseur à la position souhaitée
        // - choisit une couleur pour la pomme
        // - écrit le caractère correspondant à la pomme
        // - remet à zéro la couleur pour les prochaines utilisations
        write!(
            self.stdout,
            "{}{}{}{}",
            cursor::Goto(self.food.x, self.food.y),
            color::Fg(color::Red),
            FOOD_CHAR,
            color::Fg(color::Reset)
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }

    /// Draw snake using char c
    /// (if c = ' ' it will remove it from the screen)
    fn draw_snake_with_char(&mut self, c: char, snake: Vec<Point>, own: bool) {
        // Select color
        if own {
            write!(self.stdout, "{}", color::Fg(color::Red)).unwrap();
        } else {
            write!(self.stdout, "{}", color::Fg(color::Yellow)).unwrap();
        }
        self.stdout.flush().unwrap();
        // Add snake
        for p in snake.iter() {
            write!(
                self.stdout,
                "{}{}",
                cursor::Goto(p.x, p.y),
                c,
            ).unwrap();
        }
        // Reset color
        write!(
            self.stdout,
            "{}{}",
            cursor::Goto(0, self.field.len() as u16 + 1),
            color::Fg(color::Reset)
        ).unwrap();
        self.stdout.flush().unwrap();
    }

    /// Draw snakes using SNAKE_CHAR
    pub fn draw_snakes(&mut self) {
        for id in 0..self.snakes.len() {
            self.draw_snake_with_char(SNAKE_CHAR, self.snakes[id].clone(), self.id == id);
        }
    }

    /// Clear snakes
    pub fn clear_snakes(&mut self) {
        for id in 0..self.snakes.len() {
            self.draw_snake_with_char(' ', self.snakes[id].clone(), false);
        }
    }

    /// Update snakes positions
    pub fn update(&mut self, _snakes: Vec<Vec<Point>>) {
        let mut snakes: Vec<Vec<Point>> = vec![];
        for snake in _snakes.into_iter() {
            snakes.push(snake);
        }
        self.snakes = snakes;
    }

    /// Get last keyboard entry if there is one
    fn get_last_key_event(&mut self) -> Option<Event> {
        let mut prev: Option<Event> = None;
        loop {
            match self.stdin.next() {
                Some(b) => {
                    match parse_event(b.unwrap(), &mut self.stdin) {
                        Ok(e) => prev = Some(e),
                        _ => (),
                    }
                },
                None => break
            }
        }
        prev
    }

    /// Handle user inputs during game
    pub fn handle_input(&mut self) {
        // Get last keyboard entry if there is one
        let prev = self.get_last_key_event();
        // Update snake direction depending on last keyboard entry
        match prev {
            Some(e) => {
                match e {
                    Event::Key(key) => {
                        match key {
                            Key::Up => self.direction = Direction::Up,
                            Key::Down => self.direction = Direction::Down,
                            Key::Left => self.direction = Direction::Left,
                            Key::Right => self.direction = Direction::Right,
                            Key::Char('q') => self.killed = true,
                            _ => ()
                        }
                    },
                    _ => ()
                }
            },
            None => ()
        }
    }

    /// Handle force start event to start the game with less than 4 players
    pub fn force_start(&mut self) -> bool {
        // Get last keyboard entry if there is one
        let prev = self.get_last_key_event();
        // Update snake direction depending on last keyboard entry
        match prev {
            Some(e) => {
                match e {
                    Event::Key(key) => {
                        match key {
                            Key::Char('\n') => true,
                            _ => false,
                        }
                    },
                    _ => false,
                }
            },
            None => false,
        }
    }
}

/// Init the game's field
/// Draw field's borders and put an empty char otherwise
pub fn init_field(width: usize, height: usize) -> Vec<Vec<char>> {
    let mut field: Vec<Vec<char>> = Vec::with_capacity(height);

    // Border line content
    let mut border_line: Vec<char> = Vec::with_capacity(width);
    for _ in 0..(width) {
        border_line.push(BORDER_CHAR);
    }
    // Inner line content
    let mut inner_line: Vec<char> = Vec::with_capacity(width);
    inner_line.push(BORDER_CHAR);
    for _ in 1..(width - 1) {
        inner_line.push(' ');
    }
    inner_line.push(BORDER_CHAR);

    // Fill field
    field.push(border_line.clone());
    for _ in 1..(height - 1) {
        field.push(inner_line.clone());
    }
    field.push(border_line.clone());

    return field;
}
