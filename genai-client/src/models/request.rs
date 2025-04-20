use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    // generationConfig: Option<GenerationConfig>, // Example for future addition
    // safetySettings: Option<Vec<SafetySetting>>, // Example for future addition
}

#[derive(Serialize, Debug)]
pub struct Content {
    pub parts: Vec<Part>,
    // role: Option<String>, // Example for future addition
}

#[derive(Serialize, Debug)]
pub struct Part {
    pub text: String,
    // Add other part types later e.g.:
    // pub inline_data: Option<Blob>,
    // pub function_call: Option<FunctionCall>,
}
