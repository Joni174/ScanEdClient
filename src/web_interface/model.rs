use serde::{Deserialize};

#[derive(Deserialize)]
struct Auftrag {
    runde1: i32,
    runde2: i32,
    runde3: i32
}