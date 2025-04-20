use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    // TODO: Add promptFeedback etc.
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: ContentResponse,
    // TODO: Add finishReason, safetyRatings etc.
}

#[derive(Deserialize, Debug)]
pub struct ContentResponse {
    pub parts: Vec<PartResponse>,
    #[serde(rename = "role")] // Map JSON field "role" to Rust field "_role"
    pub _role: String,
}

#[derive(Deserialize, Debug)]
pub struct PartResponse {
    pub text: String,
    // TODO: Add other part types
}
