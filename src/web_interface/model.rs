use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Auftrag {
    input_runde1: String,
    input_runde2: String,
    input_runde3: String,
    input_hostname: String
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum PageForm {
    Auftrag(Auftrag),
    None
}

impl Auftrag {
    pub fn into_vec(self) -> Vec<i32> {
        vec![self.input_runde1.parse::<i32>().unwrap(),
             self.input_runde2.parse::<i32>().unwrap(),
             self.input_runde3.parse::<i32>().unwrap()]
    }

    pub fn get_url(&self) -> &String {
        &self.input_hostname
    }
}

#[derive(Serialize, Clone)]
pub enum ImageTakingStatus {
    Indifferent,
    ImageReady,
    ProcessFinished
}