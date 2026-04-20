use boltffi::*;
use demo_bench_macros::benchmark_candidate;

use crate::records::blittable::Point;

/// A geometric shape where each variant carries different data.
#[data]
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Circle {
        radius: f64,
    },
    Rectangle {
        width: f64,
        height: f64,
    },
    /// Triangle defined by three vertices.
    Triangle {
        a: Point,
        b: Point,
        c: Point,
    },
    Point,
}

#[data(impl)]
impl Shape {
    pub fn new(radius: f64) -> Self {
        Shape::Circle { radius }
    }

    pub fn unit_circle() -> Self {
        Shape::Circle { radius: 1.0 }
    }

    pub fn square(side: f64) -> Self {
        Shape::Rectangle {
            width: side,
            height: side,
        }
    }

    pub fn try_circle(radius: f64) -> Result<Self, String> {
        if radius <= 0.0 {
            Err("radius must be positive".to_string())
        } else {
            Ok(Shape::Circle { radius })
        }
    }

    pub fn area(&self) -> f64 {
        match self {
            Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
            Shape::Rectangle { width, height } => width * height,
            Shape::Triangle { a, b, c } => {
                ((a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)) / 2.0).abs()
            }
            Shape::Point => 0.0,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Shape::Circle { radius } => format!("circle r={}", radius),
            Shape::Rectangle { width, height } => format!("rect {}x{}", width, height),
            Shape::Triangle { .. } => "triangle".to_string(),
            Shape::Point => "point".to_string(),
        }
    }

    pub fn variant_count() -> u32 {
        4
    }
}

#[export]
pub fn echo_shape(s: Shape) -> Shape {
    s
}

#[export]
pub fn make_circle(radius: f64) -> Shape {
    Shape::Circle { radius }
}

#[export]
pub fn make_rectangle(width: f64, height: f64) -> Shape {
    Shape::Rectangle { width, height }
}

#[export]
pub fn echo_vec_shape(values: Vec<Shape>) -> Vec<Shape> {
    values
}

#[data]
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Text {
        body: String,
    },
    Image {
        url: String,
        width: u32,
        height: u32,
    },
    Ping,
}

#[export]
pub fn echo_message(m: Message) -> Message {
    m
}

#[export]
pub fn message_summary(m: Message) -> String {
    match m {
        Message::Text { body } => format!("text: {}", body),
        Message::Image { url, width, height } => format!("image: {}x{} at {}", width, height, url),
        Message::Ping => "ping".to_string(),
    }
}

#[data]
#[derive(Clone, Debug, PartialEq)]
pub enum Animal {
    Dog { name: String, breed: String },
    Cat { name: String, indoor: bool },
    Fish { count: u32 },
}

#[export]
pub fn echo_animal(a: Animal) -> Animal {
    a
}

#[export]
pub fn animal_name(a: Animal) -> String {
    match a {
        Animal::Dog { name, .. } | Animal::Cat { name, .. } => name,
        Animal::Fish { count } => format!("{} fish", count),
    }
}

#[benchmark_candidate(enum, uniffi)]
#[data]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress { progress: i32 },
    Completed { result: i32 },
    Failed { error_code: i32, retry_count: i32 },
}

#[export]
#[benchmark_candidate(function, uniffi)]
pub fn get_status_progress(status: TaskStatus) -> i32 {
    match status {
        TaskStatus::Pending => 0,
        TaskStatus::InProgress { progress } => progress,
        TaskStatus::Completed { result } => result,
        TaskStatus::Failed { error_code, .. } => error_code,
    }
}

#[export]
#[benchmark_candidate(function, uniffi)]
pub fn is_status_complete(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Completed { .. })
}
