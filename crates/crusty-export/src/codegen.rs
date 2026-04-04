//! Code generation for HTTP requests in multiple languages.
//!
//! Generates executable code snippets from a `RequestDefinition`
//! for various languages and HTTP libraries.

use crusty_core::request::{HttpMethod, KeyValue, RequestBody, RequestDefinition};

/// Supported target languages for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Rust (reqwest)
    Rust,
    /// Python (requests)
    Python,
    /// JavaScript (fetch)
    JavaScript,
    /// Go (net/http)
    Go,
    /// cURL command
    Curl,
}

impl Language {
    /// All supported languages.
    pub fn all() -> &'static [Language] {
        &[
            Self::Curl,
            Self::Rust,
            Self::Python,
            Self::JavaScript,
            Self::Go,
        ]
    }

    /// Display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rust => "Rust (reqwest)",
            Self::Python => "Python (requests)",
            Self::JavaScript => "JavaScript (fetch)",
            Self::Go => "Go (net/http)",
            Self::Curl => "cURL",
        }
    }
}

/// Generate code for a request in the given language.
pub fn generate(def: &RequestDefinition, lang: Language) -> String {
    match lang {
        Language::Curl => crate::curl::export(def),
        Language::Rust => gen_rust(def),
        Language::Python => gen_python(def),
        Language::JavaScript => gen_javascript(def),
        Language::Go => gen_go(def),
    }
}

fn gen_rust(def: &RequestDefinition) -> String {
    let method = match def.method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Patch => "patch",
        HttpMethod::Delete => "delete",
        HttpMethod::Head => "head",
        _ => "get",
    };

    let mut code = String::new();
    code.push_str("let client = reqwest::Client::new();\n");
    code.push_str(&format!(
        "let response = client.{method}(\"{}\")\n",
        def.url
    ));

    for h in &def.headers {
        if h.enabled {
            code.push_str(&format!(
                "    .header(\"{}\", \"{}\")\n",
                escape_str(&h.key),
                escape_str(&h.value)
            ));
        }
    }

    match &def.body {
        RequestBody::Json(json) => {
            code.push_str(&format!("    .body(r#\"{}\"#)\n", json));
        }
        RequestBody::Raw { content, .. } => {
            code.push_str(&format!("    .body(\"{}\")\n", escape_str(content)));
        }
        _ => {}
    }

    code.push_str("    .send()\n");
    code.push_str("    .await?;\n");
    code
}

fn gen_python(def: &RequestDefinition) -> String {
    let method = def.method.as_str().lower();

    let mut code = String::new();
    code.push_str("import requests\n\n");

    // Headers
    let enabled_headers: Vec<&KeyValue> = def.headers.iter().filter(|h| h.enabled).collect();
    if !enabled_headers.is_empty() {
        code.push_str("headers = {\n");
        for h in &enabled_headers {
            code.push_str(&format!(
                "    \"{}\": \"{}\",\n",
                escape_str(&h.key),
                escape_str(&h.value)
            ));
        }
        code.push_str("}\n\n");
    }

    // Body
    let has_body = matches!(&def.body, RequestBody::Json(_) | RequestBody::Raw { .. });
    if has_body {
        match &def.body {
            RequestBody::Json(json) => {
                code.push_str(&format!("data = '{}'\n\n", escape_str(json)));
            }
            RequestBody::Raw { content, .. } => {
                code.push_str(&format!("data = '{}'\n\n", escape_str(content)));
            }
            _ => {}
        }
    }

    // Request
    code.push_str(&format!("response = requests.{}(\n", method));
    code.push_str(&format!("    \"{}\",\n", def.url));
    if !enabled_headers.is_empty() {
        code.push_str("    headers=headers,\n");
    }
    if has_body {
        code.push_str("    data=data,\n");
    }
    code.push_str(")\n\n");
    code.push_str("print(response.status_code)\n");
    code.push_str("print(response.text)\n");
    code
}

fn gen_javascript(def: &RequestDefinition) -> String {
    let mut code = String::new();

    // Headers
    let enabled_headers: Vec<&KeyValue> = def.headers.iter().filter(|h| h.enabled).collect();

    code.push_str(&format!(
        "const response = await fetch(\"{}\", {{\n",
        def.url
    ));
    code.push_str(&format!("  method: \"{}\",\n", def.method.as_str()));

    if !enabled_headers.is_empty() {
        code.push_str("  headers: {\n");
        for h in &enabled_headers {
            code.push_str(&format!(
                "    \"{}\": \"{}\",\n",
                escape_str(&h.key),
                escape_str(&h.value)
            ));
        }
        code.push_str("  },\n");
    }

    match &def.body {
        RequestBody::Json(json) => {
            code.push_str(&format!("  body: JSON.stringify({}),\n", json));
        }
        RequestBody::Raw { content, .. } => {
            code.push_str(&format!("  body: \"{}\",\n", escape_str(content)));
        }
        _ => {}
    }

    code.push_str("});\n\n");
    code.push_str("const data = await response.json();\n");
    code.push_str("console.log(data);\n");
    code
}

fn gen_go(def: &RequestDefinition) -> String {
    let mut code = String::new();
    code.push_str("package main\n\n");
    code.push_str("import (\n");
    code.push_str("    \"fmt\"\n");
    code.push_str("    \"io\"\n");
    code.push_str("    \"net/http\"\n");

    let has_body = matches!(&def.body, RequestBody::Json(_) | RequestBody::Raw { .. });
    if has_body {
        code.push_str("    \"strings\"\n");
    }
    code.push_str(")\n\n");

    code.push_str("func main() {\n");

    if has_body {
        let body_str = match &def.body {
            RequestBody::Json(json) => json.clone(),
            RequestBody::Raw { content, .. } => content.clone(),
            _ => String::new(),
        };
        code.push_str(&format!("    body := strings.NewReader(`{}`)\n", body_str));
        code.push_str(&format!(
            "    req, err := http.NewRequest(\"{}\", \"{}\", body)\n",
            def.method.as_str(),
            def.url
        ));
    } else {
        code.push_str(&format!(
            "    req, err := http.NewRequest(\"{}\", \"{}\", nil)\n",
            def.method.as_str(),
            def.url
        ));
    }

    code.push_str("    if err != nil {\n");
    code.push_str("        panic(err)\n");
    code.push_str("    }\n\n");

    for h in &def.headers {
        if h.enabled {
            code.push_str(&format!(
                "    req.Header.Set(\"{}\", \"{}\")\n",
                escape_str(&h.key),
                escape_str(&h.value)
            ));
        }
    }

    code.push_str("\n    client := &http.Client{}\n");
    code.push_str("    resp, err := client.Do(req)\n");
    code.push_str("    if err != nil {\n");
    code.push_str("        panic(err)\n");
    code.push_str("    }\n");
    code.push_str("    defer resp.Body.Close()\n\n");
    code.push_str("    respBody, _ := io.ReadAll(resp.Body)\n");
    code.push_str("    fmt.Println(string(respBody))\n");
    code.push_str("}\n");
    code
}

/// Helper trait to lowercase a string (for method names).
trait StrLower {
    fn lower(&self) -> String;
}

impl StrLower for &str {
    fn lower(&self) -> String {
        self.to_lowercase()
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_curl() {
        let def = RequestDefinition::new("Test", "https://api.example.com");
        let code = generate(&def, Language::Curl);
        assert!(code.contains("curl"));
        assert!(code.contains("https://api.example.com"));
    }

    #[test]
    fn test_gen_rust() {
        let mut def = RequestDefinition::new("Test", "https://api.example.com");
        def.method = HttpMethod::Post;
        def.headers
            .push(KeyValue::new("Content-Type", "application/json"));
        def.body = RequestBody::Json(r#"{"key":"value"}"#.to_string());

        let code = generate(&def, Language::Rust);
        assert!(code.contains("client.post"));
        assert!(code.contains("Content-Type"));
        assert!(code.contains(r#"{"key":"value"}"#));
    }

    #[test]
    fn test_gen_python() {
        let mut def = RequestDefinition::new("Test", "https://api.example.com");
        def.method = HttpMethod::Get;

        let code = generate(&def, Language::Python);
        assert!(code.contains("import requests"));
        assert!(code.contains("requests.get"));
    }

    #[test]
    fn test_gen_javascript() {
        let def = RequestDefinition::new("Test", "https://api.example.com/data");
        let code = generate(&def, Language::JavaScript);
        assert!(code.contains("fetch"));
        assert!(code.contains("https://api.example.com/data"));
    }

    #[test]
    fn test_gen_go() {
        let mut def = RequestDefinition::new("Test", "https://api.example.com");
        def.method = HttpMethod::Delete;

        let code = generate(&def, Language::Go);
        assert!(code.contains("net/http"));
        assert!(code.contains("DELETE"));
    }
}
