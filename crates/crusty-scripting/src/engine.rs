//! Rhai-based scripting engine.
//!
//! Provides a sandboxed execution environment for pre/post-request scripts.

use crate::context::{PostRequestContext, PreRequestContext, ScriptResult, TestResult};
use crate::error::ScriptError;
use rhai::{Dynamic, Engine, Scope};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The scripting engine wrapping Rhai.
pub struct ScriptEngine;

impl ScriptEngine {
    /// Create a new scripting engine.
    pub fn new() -> Self {
        Self
    }

    /// Run a pre-request script.
    ///
    /// The script can modify variables and headers.
    pub fn run_pre_request(
        &self,
        script: &str,
        ctx: &PreRequestContext,
    ) -> Result<ScriptResult, ScriptError> {
        let logs: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let variables: Rc<RefCell<HashMap<String, String>>> =
            Rc::new(RefCell::new(ctx.variables.clone()));

        let mut engine = new_engine();
        let mut scope = Scope::new();
        scope.push("url", ctx.url.clone());
        scope.push("method", ctx.method.clone());

        let headers_map = hashmap_to_rhai(&ctx.headers);
        scope.push("headers", headers_map);

        let vars_set = Rc::clone(&variables);
        engine.register_fn("set_variable", move |key: &str, value: &str| {
            vars_set
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        });

        let vars_get = Rc::clone(&variables);
        engine.register_fn("get_variable", move |key: &str| -> String {
            vars_get.borrow().get(key).cloned().unwrap_or_default()
        });

        let logs_ref = Rc::clone(&logs);
        engine.register_fn("log", move |msg: &str| {
            logs_ref.borrow_mut().push(msg.to_string());
        });

        engine
            .run_with_scope(&mut scope, script)
            .map_err(|e: Box<rhai::EvalAltResult>| ScriptError::Runtime(e.to_string()))?;

        let result_vars = variables.borrow().clone();
        let result_logs = logs.borrow().clone();
        Ok(ScriptResult {
            variables: result_vars,
            logs: result_logs,
            tests: Vec::new(),
            all_passed: true,
        })
    }

    /// Run a post-request (test) script.
    ///
    /// The script can assert on response values and set variables.
    pub fn run_post_request(
        &self,
        script: &str,
        ctx: &PostRequestContext,
    ) -> Result<ScriptResult, ScriptError> {
        let logs: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let variables: Rc<RefCell<HashMap<String, String>>> =
            Rc::new(RefCell::new(ctx.variables.clone()));
        let tests: Rc<RefCell<Vec<TestResult>>> = Rc::new(RefCell::new(Vec::new()));

        let mut engine = new_engine();
        let mut scope = Scope::new();
        scope.push("url", ctx.url.clone());
        scope.push("method", ctx.method.clone());
        scope.push("status", ctx.status as i64);
        scope.push("status_text", ctx.status_text.clone());
        scope.push("response_body", ctx.response_body.clone());
        scope.push("response_time", ctx.response_time_ms as i64);

        let resp_headers_map = hashmap_to_rhai(&ctx.response_headers);
        scope.push("response_headers", resp_headers_map);

        let vars_set = Rc::clone(&variables);
        engine.register_fn("set_variable", move |key: &str, value: &str| {
            vars_set
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        });

        let vars_get = Rc::clone(&variables);
        engine.register_fn("get_variable", move |key: &str| -> String {
            vars_get.borrow().get(key).cloned().unwrap_or_default()
        });

        let logs_ref = Rc::clone(&logs);
        engine.register_fn("log", move |msg: &str| {
            logs_ref.borrow_mut().push(msg.to_string());
        });

        let tests_ref = Rc::clone(&tests);
        engine.register_fn("test", move |name: &str, passed: bool| {
            tests_ref.borrow_mut().push(TestResult {
                name: name.to_string(),
                passed,
                error: if passed {
                    None
                } else {
                    Some(format!("Test '{}' failed", name))
                },
            });
        });

        let tests_assert = Rc::clone(&tests);
        engine.register_fn("assert_eq", move |name: &str, a: Dynamic, b: Dynamic| {
            let passed = format!("{a}") == format!("{b}");
            tests_assert.borrow_mut().push(TestResult {
                name: name.to_string(),
                passed,
                error: if passed {
                    None
                } else {
                    Some(format!("Expected {} but got {}", b, a))
                },
            });
        });

        engine.register_fn("json_parse", |s: &str| -> Dynamic {
            match serde_json::from_str::<serde_json::Value>(s) {
                Ok(val) => json_to_rhai(&val),
                Err(_) => Dynamic::UNIT,
            }
        });

        engine
            .run_with_scope(&mut scope, script)
            .map_err(|e: Box<rhai::EvalAltResult>| ScriptError::Runtime(e.to_string()))?;

        let test_results = tests.borrow().clone();
        let all_passed = test_results.iter().all(|t| t.passed);
        let result_vars = variables.borrow().clone();
        let result_logs = logs.borrow().clone();

        Ok(ScriptResult {
            variables: result_vars,
            logs: result_logs,
            tests: test_results,
            all_passed,
        })
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn new_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_string_size(1_024 * 1_024);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);
    engine
}

fn hashmap_to_rhai(map: &HashMap<String, String>) -> rhai::Map {
    map.iter()
        .map(|(k, v)| (k.clone().into(), Dynamic::from(v.clone())))
        .collect()
}

fn json_to_rhai(val: &serde_json::Value) -> Dynamic {
    match val {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let rhai_arr: Vec<Dynamic> = arr.iter().map(json_to_rhai).collect();
            Dynamic::from(rhai_arr)
        }
        serde_json::Value::Object(obj) => {
            let rhai_map: rhai::Map = obj
                .iter()
                .map(|(k, v)| (k.clone().into(), json_to_rhai(v)))
                .collect();
            Dynamic::from(rhai_map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{PostRequestContext, PreRequestContext};

    fn default_pre_ctx() -> PreRequestContext {
        PreRequestContext {
            url: "https://api.example.com/users".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            variables: HashMap::from([
                ("host".to_string(), "api.example.com".to_string()),
                ("token".to_string(), "abc123".to_string()),
            ]),
        }
    }

    fn default_post_ctx() -> PostRequestContext {
        PostRequestContext {
            url: "https://api.example.com/users".to_string(),
            method: "GET".to_string(),
            status: 200,
            status_text: "OK".to_string(),
            response_headers: HashMap::from([(
                "content-type".to_string(),
                "application/json".to_string(),
            )]),
            response_body: r#"{"users":[{"id":1,"name":"Alice"}]}"#.to_string(),
            response_time_ms: 150,
            variables: HashMap::new(),
        }
    }

    #[test]
    fn test_pre_request_set_variable() {
        let engine = ScriptEngine::new();
        let ctx = default_pre_ctx();
        let result = engine
            .run_pre_request(r#"set_variable("new_var", "hello");"#, &ctx)
            .unwrap();
        assert_eq!(result.variables.get("new_var").unwrap(), "hello");
        // Original variables should still be there
        assert_eq!(result.variables.get("host").unwrap(), "api.example.com");
    }

    #[test]
    fn test_pre_request_get_variable() {
        let engine = ScriptEngine::new();
        let ctx = default_pre_ctx();
        let result = engine
            .run_pre_request(r#"let val = get_variable("token"); log(val);"#, &ctx)
            .unwrap();
        assert_eq!(result.logs, vec!["abc123"]);
    }

    #[test]
    fn test_pre_request_access_url() {
        let engine = ScriptEngine::new();
        let ctx = default_pre_ctx();
        let result = engine.run_pre_request(r#"log(url);"#, &ctx).unwrap();
        assert_eq!(result.logs, vec!["https://api.example.com/users"]);
    }

    #[test]
    fn test_post_request_status_check() {
        let engine = ScriptEngine::new();
        let ctx = default_post_ctx();
        let result = engine
            .run_post_request(r#"test("Status is 200", status == 200);"#, &ctx)
            .unwrap();
        assert_eq!(result.tests.len(), 1);
        assert!(result.tests[0].passed);
        assert!(result.all_passed);
    }

    #[test]
    fn test_post_request_failing_test() {
        let engine = ScriptEngine::new();
        let ctx = default_post_ctx();
        let result = engine
            .run_post_request(r#"test("Status is 404", status == 404);"#, &ctx)
            .unwrap();
        assert_eq!(result.tests.len(), 1);
        assert!(!result.tests[0].passed);
        assert!(!result.all_passed);
    }

    #[test]
    fn test_post_request_json_parse() {
        let engine = ScriptEngine::new();
        let ctx = default_post_ctx();
        let result = engine
            .run_post_request(
                r#"
                let body = json_parse(response_body);
                let users = body["users"];
                test("Has users", users.len() > 0);
                log(users[0]["name"]);
                "#,
                &ctx,
            )
            .unwrap();
        assert!(result.all_passed);
        assert_eq!(result.logs, vec!["Alice"]);
    }

    #[test]
    fn test_post_request_response_time() {
        let engine = ScriptEngine::new();
        let ctx = default_post_ctx();
        let result = engine
            .run_post_request(r#"test("Fast response", response_time < 500);"#, &ctx)
            .unwrap();
        assert!(result.all_passed);
    }

    #[test]
    fn test_script_error_handling() {
        let engine = ScriptEngine::new();
        let ctx = default_pre_ctx();
        let result = engine.run_pre_request("let x = undefined_var;", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_eq_pass() {
        let engine = ScriptEngine::new();
        let ctx = default_post_ctx();
        let result = engine
            .run_post_request(r#"assert_eq("Status check", status, 200);"#, &ctx)
            .unwrap();
        assert!(result.tests[0].passed);
    }

    #[test]
    fn test_assert_eq_fail() {
        let engine = ScriptEngine::new();
        let ctx = default_post_ctx();
        let result = engine
            .run_post_request(r#"assert_eq("Wrong status", status, 404);"#, &ctx)
            .unwrap();
        assert!(!result.tests[0].passed);
        assert!(result.tests[0].error.as_ref().unwrap().contains("Expected"));
    }

    #[test]
    fn test_multiple_tests() {
        let engine = ScriptEngine::new();
        let ctx = default_post_ctx();
        let result = engine
            .run_post_request(
                r#"
                test("Status 200", status == 200);
                test("Has body", response_body.len() > 0);
                test("Method is GET", method == "GET");
                "#,
                &ctx,
            )
            .unwrap();
        assert_eq!(result.tests.len(), 3);
        assert!(result.all_passed);
    }
}
