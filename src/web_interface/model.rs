use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Auftrag {
    runde1: i32,
    runde2: i32,
    runde3: i32,
    url: String
}

impl Auftrag {
    pub fn into_vec(self) -> Vec<i32> {
        vec![self.runde1, self.runde2, self.runde3]
    }

    pub fn get_url(&self) -> &String {
        &self.url
    }
}

#[derive(Serialize, Clone)]
pub enum ImageTakingStatus {
    Indifferent,
    ImageReady,
    ProcessFinished
}