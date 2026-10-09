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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryParam {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultipartFile {
    pub name: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultipartBody {
    pub fields: Vec<FormField>,
    pub files: Vec<MultipartFile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum RequestBody {
    Json(serde_json::Value),
    Text(String),
    #[serde(rename = "form-urlencoded")]
    FormUrlEncoded(Vec<FormField>),
    Multipart(MultipartBody),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RequestAuth {
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    #[serde(rename = "apiKey")]
    ApiKey {
        name: String,
        value: String,
        location: ApiKeyLocation,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AssertionDefinition {
    StatusEquals { expected: u16 },
    HeaderContains { name: String, value: String },
    BodyContains { value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestDefinition {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub query: Vec<QueryParam>,
    pub headers: Vec<HeaderEntry>,
    #[serde(default)]
    pub cookies: Vec<CookieEntry>,
    pub body: Option<RequestBody>,
    #[serde(default)]
    pub auth: Option<RequestAuth>,
    #[serde(default)]
    pub assertions: Vec<AssertionDefinition>,
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
            cookies: vec![],
            body: Some(RequestBody::Json(json!({
                "amount": 100
            }))),
            auth: Some(RequestAuth::Bearer {
                token: "{{secret.accessToken}}".to_string(),
            }),
            assertions: vec![AssertionDefinition::StatusEquals { expected: 201 }],
        };

        let serialized = serde_json::to_value(&request).unwrap();

        assert_eq!(serialized["method"], "POST");
        assert_eq!(serialized["name"], "Criar pagamento");
        assert_eq!(serialized["body"]["type"], "json");
        assert_eq!(serialized["body"]["value"]["amount"], 100);
        assert_eq!(serialized["auth"]["type"], "bearer");
        assert_eq!(serialized["assertions"][0]["type"], "statusEquals");
    }

    #[test]
    fn aceita_request_antiga_sem_auth() {
        let request: RequestDefinition = serde_json::from_value(serde_json::json!({
            "id": "health",
            "name": "Health",
            "method": "GET",
            "url": "http://localhost:3000/health",
            "query": [],
            "headers": [],
            "body": null
        }))
        .unwrap();

        assert!(request.auth.is_none());
        assert!(request.cookies.is_empty());
        assert!(request.assertions.is_empty());
    }
}
