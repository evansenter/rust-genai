use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    // TODO: Add generationConfig, safetySettings, etc. if needed
}

#[derive(Serialize, Debug)]
pub struct Content {
    pub parts: Vec<Part>,
    // TODO: Add role if needed, defaults to "user" usually for simple requests
}

#[derive(Serialize, Debug)]
pub struct Part {
    pub text: String,
    // TODO: Add other part types like inline_data, function_call etc.
}
