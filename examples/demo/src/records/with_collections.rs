use boltffi::*;
use demo_bench_macros::benchmark_candidate;

use crate::records::blittable::Point;
use crate::records::with_strings::Person;

#[data]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Polygon {
    pub points: Vec<Point>,
}

#[export]
pub fn echo_polygon(p: Polygon) -> Polygon {
    p
}

#[export]
pub fn make_polygon(points: Vec<Point>) -> Polygon {
    Polygon { points }
}

#[export]
pub fn polygon_vertex_count(p: Polygon) -> u32 {
    p.points.len() as u32
}

#[export]
pub fn polygon_centroid(p: Polygon) -> Point {
    if p.points.is_empty() {
        return Point { x: 0.0, y: 0.0 };
    }
    let count = p.points.len() as f64;
    let sum_x: f64 = p.points.iter().map(|pt| pt.x).sum();
    let sum_y: f64 = p.points.iter().map(|pt| pt.y).sum();
    Point {
        x: sum_x / count,
        y: sum_y / count,
    }
}

#[data]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Team {
    pub name: String,
    pub members: Vec<String>,
}

#[export]
pub fn echo_team(t: Team) -> Team {
    t
}

#[export]
pub fn make_team(name: String, members: Vec<String>) -> Team {
    Team { name, members }
}

#[export]
pub fn team_size(t: Team) -> u32 {
    t.members.len() as u32
}

#[data]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Classroom {
    pub students: Vec<Person>,
}

#[export]
pub fn echo_classroom(c: Classroom) -> Classroom {
    c
}

#[export]
pub fn make_classroom(students: Vec<Person>) -> Classroom {
    Classroom { students }
}

#[data]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TaggedScores {
    pub label: String,
    pub scores: Vec<f64>,
}

#[export]
pub fn echo_tagged_scores(ts: TaggedScores) -> TaggedScores {
    ts
}

#[export]
pub fn average_score(ts: TaggedScores) -> f64 {
    if ts.scores.is_empty() {
        return 0.0;
    }
    let sum: f64 = ts.scores.iter().sum();
    sum / ts.scores.len() as f64
}

/// A heavier benchmark profile with heap-owned collections.
#[benchmark_candidate(record, uniffi)]
#[data]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchmarkUserProfile {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub bio: String,
    pub age: i32,
    pub score: f64,
    pub tags: Vec<String>,
    pub scores: Vec<i32>,
    pub is_active: bool,
}

#[export]
#[benchmark_candidate(function, uniffi)]
pub fn generate_user_profiles(count: i32) -> Vec<BenchmarkUserProfile> {
    (0..count as i64)
        .map(|index| BenchmarkUserProfile {
            id: index,
            name: format!("User {index}"),
            email: format!("user{index}@example.com"),
            bio: format!(
                "This is a bio for user {index}. It contains enough text to behave like a real payload."
            ),
            age: 20 + (index % 50) as i32,
            score: index as f64 * 1.5,
            tags: vec![
                format!("tag{}", index % 5),
                format!("category{}", index % 3),
                "common".to_string(),
            ],
            scores: vec![
                (index % 100) as i32,
                ((index + 10) % 100) as i32,
                ((index + 20) % 100) as i32,
            ],
            is_active: index % 2 == 0,
        })
        .collect()
}

#[export]
#[benchmark_candidate(function, uniffi)]
pub fn sum_user_scores(users: Vec<BenchmarkUserProfile>) -> f64 {
    users.iter().map(|user| user.score).sum()
}

#[export]
#[benchmark_candidate(function, uniffi)]
pub fn count_active_users(users: Vec<BenchmarkUserProfile>) -> i32 {
    users.iter().filter(|user| user.is_active).count() as i32
}
