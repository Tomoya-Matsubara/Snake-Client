pub mod game;
use chrono::{Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;

const SERVER_ADDR: &'static str = "127.0.0.1";
const SERVER_PORT: usize = 8080;

const LOG_FILE: &'static str = "log";

/// Stream object to store our reader and writer object
struct Stream<'a> {
    reader: BufReader<&'a TcpStream>,
    writer: BufWriter<&'a TcpStream>,
}

/// Game configuration
#[derive(Deserialize)]
pub struct GameConfig {
    id: usize,
    width: usize,
    height: usize,
    snakes: Vec<Vec<game::Point>>,
    food: game::Point,
}
/// Direction message
#[derive(Serialize)]
struct DirectionMessage {
    direction: game::Direction,
}
/// Snake positions for next turn
#[derive(Deserialize)]
struct TurnMessage {
    id: usize,
    food: game::Point,
    snakes: Vec<Vec<game::Point>>,
}
/// Game events
#[derive(Deserialize)]
struct EventMessage {
    event: game::GameEvent,
}
/// Game state to notify ready/playing/lost
#[derive(Deserialize)]
struct StateMessage {
    state: game::GameState,
}
impl PartialEq<game::GameState> for StateMessage {
    fn eq(&self, other: &game::GameState) -> bool {
        self.state == *other
    }
}
/// Force start message
#[derive(Serialize)]
pub struct ForceStartMessage {
    pub force_start: bool,
}

/// Initialize a connection with the server
fn connect<'a>() -> Result<TcpStream, String> {
    let addr = format!("{}:{}", SERVER_ADDR, SERVER_PORT);
    return match TcpStream::connect(addr) {
        Ok(stream) => Ok(stream),
        Err(e) => Err(format!("Failed to connect to server: {}", e)),
    };
}

/// Serialize object and send it as a json to the server
fn send<T>(stream: &mut Stream, object: T)
where
    T: Serialize,
{
    let payload = format!("{}\n", serde_json::to_string(&object).unwrap());
    stream.writer.write(payload.as_bytes()).unwrap();
    stream.writer.flush().unwrap();
}

/// Wait for server message, read it and deserialize it depeding on T
fn receive<'a, T>(stream: &mut Stream, response: &'a mut String) -> T
where
    T: Deserialize<'a>,
{
    stream.reader.read_line(response).unwrap();
    serde_json::from_str::<'a, T>(&response[..]).unwrap()
}

/// Log function
fn log(s: &str) {
    if let Ok(mut file) = OpenOptions::new().append(true).open(LOG_FILE) {
        let now = Utc::now();
        let line = format!("[{}:{}:{}] {}\n", now.hour(), now.minute(), now.second(), s);
        file.write(line.as_bytes()).unwrap();
    }
}

fn main() {
    // Reset log file
    File::create(LOG_FILE).unwrap();

    // Connect to the server
    let s = connect().unwrap();
    let mut stream = Stream {
        reader: BufReader::new(&s),
        writer: BufWriter::new(&s),
    };
    log("Connection initialized successfully");

    let mut game = game::Game::empty();

    // Enter lobby
    log("Entering Lobby");
    println!("Press ENTER to start game with less than 4 players");
    loop {
        let mut response = String::new();
        let event: EventMessage = receive(&mut stream, &mut response);
        match event.event {
            game::GameEvent::Start => break,
            game::GameEvent::WaitInLobby => (),
            // If it's not a new turn something went wrong, so exit game
            _ => panic!("Wrong server message received"),
        }
        // If force start game, send start message to the server
        send(&mut stream, ForceStartMessage { force_start: game.force_start() });
    }

    log("Starting game");

    // Read GameConfig from the server
    let mut response = String::new();
    let config: GameConfig = receive(&mut stream, &mut response);
    log(&format!("Received game configuration: {}", response)[..]);

    // Init game
    log("Initializing game");
    game.set_config(config);
    game.draw_field();
    game.draw_food();
    game.draw_snakes();

    // Enter play state
    loop {
        // Wait new turn event before making this call, it allows a better sync with the server
        let mut response = String::new();
        let event: EventMessage = receive(&mut stream, &mut response);
        match event.event {
            game::GameEvent::NewTurn => (),
            _ => break, // If it's not a new turn something went wrong, so exit game
        }
        // Handle user inputs
        game.handle_input();
        // If user killed the game, exit
        if game.killed {
            break;
        }
        log(&format!("Current direction: {}", game.direction.clone())[..]);
        // Send current direction to the server
        log("Send user direction to the server");
        send(
            &mut stream,
            DirectionMessage {
                direction: game.direction.clone(),
            },
        );
        // Wait server response with updated game
        let mut response = String::new();
        let turn: TurnMessage = receive(&mut stream, &mut response);
        log(&format!("Received next turn data: {}", response)[..]);
        // Clear old snake positions and update new ones
        game.id = turn.id;
        game.food = turn.food;
        game.clear_snakes();
        game.draw_food();
        game.update(turn.snakes);
        game.draw_snakes();
        // Check if the game is over
        response = String::new();
        let state: StateMessage = receive(&mut stream, &mut response);
        log(&format!("Received game state: {}", response)[..]);
        if state.state == game::GameState::Lost {
            println!("You lose");
            log("You lose");
            break;
        }
    }
}
