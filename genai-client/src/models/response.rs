use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    // pub prompt_feedback: Option<PromptFeedback>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: ContentResponse,
    // pub finish_reason: Option<String>,
    // pub safety_ratings: Option<Vec<SafetyRating>>,
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
    // Add other part types later
}
