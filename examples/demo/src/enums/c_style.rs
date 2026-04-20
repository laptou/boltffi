use boltffi::*;
use demo_bench_macros::benchmark_candidate;

/// Lifecycle status of an entity.
#[data]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Active,
    Inactive,
    Pending,
}

#[export]
pub fn echo_status(s: Status) -> Status {
    s
}

#[export]
pub fn status_to_string(s: Status) -> String {
    match s {
        Status::Active => "active".to_string(),
        Status::Inactive => "inactive".to_string(),
        Status::Pending => "pending".to_string(),
    }
}

#[export]
pub fn is_active(s: Status) -> bool {
    matches!(s, Status::Active)
}

#[export]
pub fn echo_vec_status(values: Vec<Status>) -> Vec<Status> {
    values
}

#[benchmark_candidate(enum, uniffi, wasm_bindgen)]
#[data]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    North,
    South,
    East,
    West,
}

#[data(impl)]
impl Direction {
    pub fn new(raw: i32) -> Self {
        match raw {
            0 => Direction::North,
            1 => Direction::South,
            2 => Direction::East,
            3 => Direction::West,
            _ => Direction::North,
        }
    }

    pub fn cardinal() -> Self {
        Direction::North
    }

    pub fn from_degrees(degrees: f64) -> Self {
        let normalized = ((degrees % 360.0) + 360.0) % 360.0;
        if normalized < 45.0 || normalized >= 315.0 {
            Direction::North
        } else if normalized < 135.0 {
            Direction::East
        } else if normalized < 225.0 {
            Direction::South
        } else {
            Direction::West
        }
    }

    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    pub fn is_horizontal(&self) -> bool {
        matches!(self, Direction::East | Direction::West)
    }

    pub fn label(&self) -> String {
        match self {
            Direction::North => "N".to_string(),
            Direction::South => "S".to_string(),
            Direction::East => "E".to_string(),
            Direction::West => "W".to_string(),
        }
    }

    pub fn count() -> u32 {
        4
    }
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn echo_direction(d: Direction) -> Direction {
    d
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn opposite_direction(d: Direction) -> Direction {
    match d {
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::East => Direction::West,
        Direction::West => Direction::East,
    }
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn direction_to_degrees(direction: Direction) -> i32 {
    match direction {
        Direction::North => 0,
        Direction::East => 90,
        Direction::South => 180,
        Direction::West => 270,
    }
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn generate_directions(count: i32) -> Vec<Direction> {
    let directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    (0..count as usize)
        .map(|index| directions[index % directions.len()])
        .collect()
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn count_north(directions: Vec<Direction>) -> i32 {
    directions
        .iter()
        .filter(|direction| matches!(direction, Direction::North))
        .count() as i32
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn find_direction(id: i32) -> Option<Direction> {
    match id {
        0 => Some(Direction::North),
        1 => Some(Direction::East),
        2 => Some(Direction::South),
        3 => Some(Direction::West),
        _ => None,
    }
}

#[export]
#[benchmark_candidate(function, uniffi)]
pub fn find_directions(count: i32) -> Option<Vec<Direction>> {
    if count > 0 {
        Some(generate_directions(count))
    } else {
        None
    }
}
