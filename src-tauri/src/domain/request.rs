use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParam {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum RequestBody {
    Json(serde_json::Value),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestDefinition {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub query: Vec<QueryParam>,
    pub headers: Vec<HeaderEntry>,
    pub body: Option<RequestBody>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializa_request_json() {
        let request = RequestDefinition {
            id: "request-1".to_string(),
            name: "Criar pagamento".to_string(),
            method: HttpMethod::Post,
            url: "https://api.example.com/payments".to_string(),
            query: vec![],
            headers: vec![HeaderEntry {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            }],
            body: Some(RequestBody::Json(json!({
                "amount": 100
            }))),
        };

        let serialized = serde_json::to_value(&request).unwrap();

        assert_eq!(serialized["method"], "POST");
        assert_eq!(serialized["name"], "Criar pagamento");
        assert_eq!(
            serialized["body"]["type"],
            "json"
        );
        assert_eq!(serialized["body"]["value"]["amount"], 100);
    }
}