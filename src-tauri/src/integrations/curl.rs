use reqwest::Url;
use serde::Serialize;

use crate::domain::request::{
    CookieEntry, FormField, HeaderEntry, HttpMethod, MultipartBody, MultipartFile, QueryParam,
    RequestAuth, RequestBody, RequestDefinition,
};

const MAX_CURL_COMMAND_BYTES: usize = 256 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurlImportPreview {
    pub request: RequestDefinition,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurlImportError {
    pub kind: String,
    pub message: String,
}

impl CurlImportError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

#[derive(Default)]
struct CurlParts {
    url: Option<String>,
    method: Option<HttpMethod>,
    headers: Vec<HeaderEntry>,
    cookies: Vec<CookieEntry>,
    auth: Option<RequestAuth>,
    data: Vec<String>,
    form_urlencoded: Vec<FormField>,
    multipart_fields: Vec<FormField>,
    multipart_files: Vec<MultipartFile>,
    use_get: bool,
    json_mode: bool,
    warnings: Vec<String>,
}

pub fn parse_command(command: &str) -> Result<CurlImportPreview, CurlImportError> {
    if command.trim().is_empty() {
        return Err(CurlImportError::new(
            "empty_command",
            "Cole um comando cURL para iniciar a importação.",
        ));
    }

    if command.len() > MAX_CURL_COMMAND_BYTES {
        return Err(CurlImportError::new(
            "command_too_large",
            format!(
                "O comando cURL excede o limite local de {} KiB.",
                MAX_CURL_COMMAND_BYTES / 1024
            ),
        ));
    }

    let tokens = tokenize(command)?;
    if tokens.is_empty() || !is_curl_program(&tokens[0]) {
        return Err(CurlImportError::new(
            "invalid_command",
            "O texto precisa começar com curl ou curl.exe.",
        ));
    }

    let mut parts = CurlParts::default();
    let mut index = 1;

    while index < tokens.len() {
        let token = &tokens[index];

        if token == "--" {
            index += 1;
            collect_positional_urls(&tokens, &mut index, &mut parts);
            break;
        }

        if token == "-G" || token == "--get" {
            parts.use_get = true;
            index += 1;
            continue;
        }

        if token == "-k" || token == "--insecure" {
            parts.warnings.push(
                "A opção --insecure não foi importada. A validação TLS do Larry permanece ativa."
                    .to_string(),
            );
            index += 1;
            continue;
        }

        if token == "-L" || token == "--location" {
            parts.warnings.push(
                "A opção --location não foi importada. Redirecionamentos serão revisados na execução."
                    .to_string(),
            );
            index += 1;
            continue;
        }

        if token == "--compressed" {
            parts.warnings.push(
                "A opção --compressed não foi importada como configuração explícita da request."
                    .to_string(),
            );
            index += 1;
            continue;
        }

        if matches!(
            token.as_str(),
            "--http1.1" | "--http2" | "--http2-prior-knowledge" | "--http3"
        ) {
            parts.warnings.push(format!(
                "A opção {} não foi importada. A seleção de protocolo será configurada em uma fase própria.",
                token
            ));
            index += 1;
            continue;
        }

        if let Some(value) = inline_or_next_value(&tokens, &mut index, token, &["-X", "--request"])?
        {
            parts.method = Some(parse_method(&value)?);
            continue;
        }

        if let Some(value) = inline_or_next_value(&tokens, &mut index, token, &["--url"])? {
            parts.url = Some(value);
            continue;
        }

        if let Some(value) = inline_or_next_value(&tokens, &mut index, token, &["-H", "--header"])?
        {
            parse_header(&value, &mut parts)?;
            continue;
        }

        if let Some(value) = inline_or_next_value(&tokens, &mut index, token, &["-b", "--cookie"])?
        {
            parse_cookies(&value, &mut parts.cookies, &mut parts.warnings);
            continue;
        }

        if let Some(value) = inline_or_next_value(&tokens, &mut index, token, &["-u", "--user"])? {
            let (username, password) = value.split_once(':').ok_or_else(|| {
                CurlImportError::new(
                    "invalid_basic_auth",
                    "A opção --user precisa estar no formato usuario:senha.",
                )
            })?;
            parts.auth = Some(RequestAuth::Basic {
                username: username.to_string(),
                password: password.to_string(),
            });
            push_credential_warning(&mut parts.warnings);
            continue;
        }

        if let Some(value) = inline_or_next_value(
            &tokens,
            &mut index,
            token,
            &[
                "-d",
                "--data",
                "--data-raw",
                "--data-binary",
                "--data-ascii",
            ],
        )? {
            if value.starts_with('@') {
                parts.warnings.push(
                    "Um body vindo de arquivo foi encontrado, mas não foi lido automaticamente."
                        .to_string(),
                );
            } else {
                parts.data.push(value);
            }
            continue;
        }

        if let Some(value) = inline_or_next_value(&tokens, &mut index, token, &["--json"])? {
            parts.json_mode = true;
            parts.data.push(value);
            continue;
        }

        if let Some(value) =
            inline_or_next_value(&tokens, &mut index, token, &["--data-urlencode"])?
        {
            let (name, field_value) = value.split_once('=').ok_or_else(|| {
                CurlImportError::new(
                    "invalid_form_field",
                    "Cada --data-urlencode precisa estar no formato nome=valor.",
                )
            })?;
            parts.form_urlencoded.push(FormField {
                name: name.to_string(),
                value: field_value.to_string(),
                enabled: true,
            });
            continue;
        }

        if let Some(value) = inline_or_next_value(
            &tokens,
            &mut index,
            token,
            &["-F", "--form", "--form-string"],
        )? {
            parse_multipart_field(&value, &mut parts)?;
            continue;
        }

        if let Some(value) =
            inline_or_next_value(&tokens, &mut index, token, &["-A", "--user-agent"])?
        {
            parts.headers.push(HeaderEntry {
                name: "User-Agent".to_string(),
                value,
                enabled: true,
            });
            continue;
        }

        if let Some(value) = inline_or_next_value(&tokens, &mut index, token, &["-e", "--referer"])?
        {
            parts.headers.push(HeaderEntry {
                name: "Referer".to_string(),
                value,
                enabled: true,
            });
            continue;
        }

        if is_unsupported_value_option(token) {
            parts.warnings.push(format!(
                "A opção {} não foi importada porque depende de arquivo, certificado ou configuração de transporte.",
                option_name(token)
            ));
            index += 1;
            if !token.contains('=') {
                index += 1;
            }
            continue;
        }

        if token.starts_with('-') {
            parts
                .warnings
                .push(format!("A opção {} não foi importada.", option_name(token)));
            index += 1;
            continue;
        }

        if parts.url.is_none() {
            parts.url = Some(token.clone());
        } else {
            parts.warnings.push(
                "Mais de uma URL foi encontrada; somente a primeira foi importada.".to_string(),
            );
        }
        index += 1;
    }

    let raw_url = parts.url.ok_or_else(|| {
        CurlImportError::new(
            "missing_url",
            "Não encontrei uma URL no comando cURL importado.",
        )
    })?;
    let (url, query) = split_url(&raw_url)?;

    let method = parts.method.unwrap_or_else(|| {
        if parts.use_get {
            HttpMethod::Get
        } else if !parts.data.is_empty()
            || !parts.form_urlencoded.is_empty()
            || !parts.multipart_fields.is_empty()
        {
            HttpMethod::Post
        } else {
            HttpMethod::Get
        }
    });

    let mut headers = parts.headers;
    if parts.json_mode && !has_header(&headers, "content-type") {
        headers.push(HeaderEntry {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
            enabled: true,
        });
    }

    let form_urlencoded_for_query = if parts.use_get {
        parts.form_urlencoded.clone()
    } else {
        Vec::new()
    };

    let body = if !parts.multipart_fields.is_empty() || !parts.multipart_files.is_empty() {
        Some(RequestBody::Multipart(MultipartBody {
            fields: parts.multipart_fields,
            files: parts.multipart_files,
        }))
    } else if !parts.form_urlencoded.is_empty() {
        Some(RequestBody::FormUrlEncoded(parts.form_urlencoded))
    } else if !parts.data.is_empty() && !parts.use_get {
        let value = parts.data.join("&");
        if parts.json_mode {
            match serde_json::from_str(&value) {
                Ok(json) => Some(RequestBody::Json(json)),
                Err(_) => {
                    parts.warnings.push(
                        "O body de --json não pôde ser interpretado como JSON e foi mantido como texto."
                            .to_string(),
                    );
                    Some(RequestBody::Text(value))
                }
            }
        } else if looks_like_json(&value) {
            match serde_json::from_str(&value) {
                Ok(json) => Some(RequestBody::Json(json)),
                Err(_) => Some(RequestBody::Text(value)),
            }
        } else {
            Some(RequestBody::Text(value))
        }
    } else {
        None
    };

    let had_body = body.is_some();
    let mut final_query = query;
    if parts.use_get && !parts.data.is_empty() {
        append_query_data(&mut final_query, &parts.data.join("&"));
    }
    if parts.use_get {
        for field in form_urlencoded_for_query {
            final_query.push(QueryParam {
                name: field.name,
                value: field.value,
                enabled: field.enabled,
            });
        }
    }

    if parts.use_get && had_body {
        parts.warnings.push(
            "Dados de uma request -G foram movidos para a query; o body não será enviado."
                .to_string(),
        );
    }

    let request = RequestDefinition {
        id: "curl-imported".to_string(),
        name: "Importada do cURL".to_string(),
        method,
        url,
        query: final_query,
        headers,
        cookies: parts.cookies,
        body: if parts.use_get { None } else { body },
        auth: parts.auth,
        assertions: vec![],
    };

    Ok(CurlImportPreview {
        request,
        warnings: parts.warnings,
    })
}

fn is_curl_program(value: &str) -> bool {
    value.eq_ignore_ascii_case("curl") || value.eq_ignore_ascii_case("curl.exe")
}

fn tokenize(command: &str) -> Result<Vec<String>, CurlImportError> {
    let normalized = command.replace("\\\r\n", "").replace("\\\n", "");
    let chars: Vec<char> = normalized.chars().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut index = 0;

    while index < chars.len() {
        let character = chars[index];

        if in_single {
            if character == '\'' {
                in_single = false;
            } else {
                current.push(character);
            }
            index += 1;
            continue;
        }

        if in_double {
            if character == '"' {
                in_double = false;
            } else if character == '\\'
                && chars.get(index + 1).is_some_and(|next| is_escapable(*next))
            {
                index += 1;
                current.push(chars[index]);
            } else {
                current.push(character);
            }
            index += 1;
            continue;
        }

        match character {
            '\'' => in_single = true,
            '"' => in_double = true,
            '\\' if chars.get(index + 1).is_some_and(|next| is_escapable(*next)) => {
                index += 1;
                current.push(chars[index]);
            }
            character if character.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
        index += 1;
    }

    if in_single || in_double {
        return Err(CurlImportError::new(
            "unterminated_quote",
            "O comando cURL possui aspas não fechadas.",
        ));
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn is_escapable(character: char) -> bool {
    character.is_whitespace() || matches!(character, '\\' | '\'' | '"')
}

fn inline_or_next_value(
    tokens: &[String],
    index: &mut usize,
    token: &str,
    options: &[&str],
) -> Result<Option<String>, CurlImportError> {
    for option in options {
        if token == *option {
            *index += 1;
            let value = tokens.get(*index).ok_or_else(|| {
                CurlImportError::new(
                    "missing_option_value",
                    format!("A opção {} precisa de um valor.", option),
                )
            })?;
            *index += 1;
            return Ok(Some(value.clone()));
        }

        let prefix = format!("{}=", option);
        if let Some(value) = token.strip_prefix(&prefix) {
            *index += 1;
            return Ok(Some(value.to_string()));
        }

        if option.starts_with('-') && !option.starts_with("--") && token.starts_with(option) {
            let value = token.strip_prefix(option).unwrap_or_default();
            if !value.is_empty() {
                *index += 1;
                return Ok(Some(value.to_string()));
            }
        }
    }

    Ok(None)
}

fn collect_positional_urls(tokens: &[String], index: &mut usize, parts: &mut CurlParts) {
    while *index < tokens.len() {
        let token = &tokens[*index];
        if parts.url.is_none() {
            parts.url = Some(token.clone());
        } else {
            parts.warnings.push(
                "Mais de uma URL foi encontrada; somente a primeira foi importada.".to_string(),
            );
        }
        *index += 1;
    }
}

fn parse_method(value: &str) -> Result<HttpMethod, CurlImportError> {
    match value.to_ascii_uppercase().as_str() {
        "GET" => Ok(HttpMethod::Get),
        "POST" => Ok(HttpMethod::Post),
        "PUT" => Ok(HttpMethod::Put),
        "PATCH" => Ok(HttpMethod::Patch),
        "DELETE" => Ok(HttpMethod::Delete),
        method => Err(CurlImportError::new(
            "unsupported_method",
            format!(
                "O método cURL {} ainda não é suportado pelo modelo do Larry.",
                method
            ),
        )),
    }
}

fn parse_header(value: &str, parts: &mut CurlParts) -> Result<(), CurlImportError> {
    let (name, header_value) = value.split_once(':').ok_or_else(|| {
        CurlImportError::new(
            "invalid_header",
            "Cada header cURL precisa estar no formato Nome: valor.",
        )
    })?;
    let name = name.trim();
    let header_value = header_value.trim();
    if name.is_empty() {
        return Err(CurlImportError::new(
            "invalid_header",
            "O nome de um header cURL não pode ficar vazio.",
        ));
    }

    if name.eq_ignore_ascii_case("cookie") {
        parse_cookies(header_value, &mut parts.cookies, &mut parts.warnings);
        return Ok(());
    }

    if name.eq_ignore_ascii_case("authorization") {
        if let Some(token) = header_value.strip_prefix("Bearer ") {
            parts.auth = Some(RequestAuth::Bearer {
                token: token.trim().to_string(),
            });
            push_credential_warning(&mut parts.warnings);
            return Ok(());
        }
    }

    if name.eq_ignore_ascii_case("content-length") {
        parts.warnings.push(
            "Content-Length foi importado como header, mas o transporte pode recalculá-lo para o body atual."
                .to_string(),
        );
    }

    parts.headers.push(HeaderEntry {
        name: name.to_string(),
        value: header_value.to_string(),
        enabled: true,
    });

    if name.eq_ignore_ascii_case("authorization") {
        push_credential_warning(&mut parts.warnings);
    }

    Ok(())
}

fn parse_cookies(value: &str, cookies: &mut Vec<CookieEntry>, warnings: &mut Vec<String>) {
    if value.starts_with('@') {
        warnings.push(
            "Um arquivo de cookies foi referenciado, mas não foi lido automaticamente.".to_string(),
        );
        return;
    }

    for pair in value.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }

        if let Some((name, cookie_value)) = pair.split_once('=') {
            cookies.push(CookieEntry {
                name: name.trim().to_string(),
                value: cookie_value.trim().to_string(),
                enabled: true,
            });
        } else {
            warnings
                .push("Um cookie foi ignorado porque não possui o formato nome=valor.".to_string());
        }
    }

    if !cookies.is_empty() {
        push_credential_warning(warnings);
    }
}

fn parse_multipart_field(value: &str, parts: &mut CurlParts) -> Result<(), CurlImportError> {
    let (name, field_value) = value.split_once('=').ok_or_else(|| {
        CurlImportError::new(
            "invalid_form_field",
            "Cada --form precisa estar no formato nome=valor.",
        )
    })?;
    let name = name.trim();
    let field_value = field_value.trim();
    if name.is_empty() {
        return Err(CurlImportError::new(
            "invalid_form_field",
            "O nome de um campo multipart não pode ficar vazio.",
        ));
    }

    if let Some(path) = field_value.strip_prefix('@') {
        parts.multipart_files.push(MultipartFile {
            name: name.to_string(),
            path: path.split(';').next().unwrap_or(path).to_string(),
            enabled: true,
        });
        if field_value.contains(';') {
            parts.warnings.push(
                "Atributos extras do campo multipart foram ignorados; o caminho do arquivo foi mantido."
                    .to_string(),
            );
        }
    } else {
        parts.multipart_fields.push(FormField {
            name: name.to_string(),
            value: field_value.to_string(),
            enabled: true,
        });
    }

    Ok(())
}

fn split_url(raw_url: &str) -> Result<(String, Vec<QueryParam>), CurlImportError> {
    let mut url = Url::parse(raw_url).map_err(|error| {
        CurlImportError::new(
            "invalid_url",
            format!("A URL do cURL não pôde ser interpretada: {}", error),
        )
    })?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(CurlImportError::new(
            "unsupported_scheme",
            "A importação de cURL suporta URLs http e https.",
        ));
    }

    let query = url
        .query_pairs()
        .map(|(name, value)| QueryParam {
            name: name.into_owned(),
            value: value.into_owned(),
            enabled: true,
        })
        .collect();
    url.set_query(None);
    Ok((url.to_string(), query))
}

fn append_query_data(query: &mut Vec<QueryParam>, data: &str) {
    for pair in data.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        query.push(QueryParam {
            name: name.to_string(),
            value: value.to_string(),
            enabled: true,
        });
    }
}

fn looks_like_json(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn has_header(headers: &[HeaderEntry], name: &str) -> bool {
    headers
        .iter()
        .any(|header| header.enabled && header.name.eq_ignore_ascii_case(name))
}

fn push_credential_warning(warnings: &mut Vec<String>) {
    let warning =
        "Credenciais foram importadas apenas para revisão. Mova valores sensíveis para um secret antes de salvar a collection.";
    if !warnings.iter().any(|item| item == warning) {
        warnings.push(warning.to_string());
    }
}

fn is_unsupported_value_option(token: &str) -> bool {
    [
        "-o",
        "--output",
        "--max-time",
        "--connect-timeout",
        "--cert",
        "--key",
        "--cacert",
        "--proxy",
        "-x",
    ]
    .iter()
    .any(|option| token == *option || token.starts_with(&format!("{}=", option)))
}

fn option_name(token: &str) -> String {
    token
        .split_once('=')
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importa_json_headers_query_e_bearer() {
        let preview = parse_command(
            r####"curl --request POST "https://api.example.com/payments?environment=sandbox" --header "Content-Type: application/json" --header "Authorization: Bearer token-123" --data '{"amount":100}'"####,
        )
        .unwrap();

        assert!(matches!(preview.request.method, HttpMethod::Post));
        assert_eq!(preview.request.url, "https://api.example.com/payments");
        assert_eq!(preview.request.query[0].name, "environment");
        assert!(matches!(preview.request.body, Some(RequestBody::Json(_))));
        assert!(matches!(
            preview.request.auth,
            Some(RequestAuth::Bearer { .. })
        ));
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("Credenciais")));
    }

    #[test]
    fn importa_get_data_como_query() {
        let preview = parse_command(
            "curl -G https://api.example.com/search --data-urlencode q=larry --data page=2",
        )
        .unwrap();

        assert!(matches!(preview.request.method, HttpMethod::Get));
        assert_eq!(preview.request.query.len(), 2);
        assert!(preview.request.body.is_none());
    }

    #[test]
    fn importa_multipart_e_preserva_caminho_windows() {
        let preview = parse_command(
            r####"curl https://api.example.com/upload -F "description=invoice" -F "file=@C:\temp\invoice.pdf""####,
        )
        .unwrap();

        let Some(RequestBody::Multipart(body)) = preview.request.body else {
            panic!("body multipart esperado");
        };
        assert_eq!(body.fields[0].value, "invoice");
        assert_eq!(body.files[0].path, r####"C:\temp\invoice.pdf"####);
    }

    #[test]
    fn aceita_multilinha_com_aspas_e_avisa_opcao_nao_importada() {
        let preview = parse_command(
            "curl \\\n+              --url 'https://api.example.com/health' \\\n+              --compressed",
        )
        .unwrap();

        assert_eq!(preview.request.url, "https://api.example.com/health");
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("compressed")));
    }

    #[test]
    fn rejeita_comando_sem_url() {
        let error = parse_command("curl --request GET").unwrap_err();
        assert_eq!(error.kind, "missing_url");
    }
}
