use std::fmt;
use std::hash::{Hash, Hasher};

use serde::de::{Error as _, Visitor};
use serde::ser::{Error as _, SerializeStruct};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

const VERSION_ERROR: &str = "JSON-RPC version must be exactly \"2.0\"";

macro_rules! jsonrpc_error_constructors {
    ($(#[$meta:meta])* $name:ident => $code:ident; $($rest:tt)*) => {
        $(#[$meta])*
        pub fn $name(message: impl Into<String>) -> Self {
            Self::new(ErrorCode::$code, message)
        }

        jsonrpc_error_constructors!($($rest)*);
    };
    () => {};
}

/// `Deserialize` and `from_value` for an envelope decoded by its `from_object`.
macro_rules! owned_value_decoding {
    ($name:ident, $kind:literal, $errors:literal) => {
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Self::from_object(Value::deserialize(deserializer)?)
            }
        }

        impl $name {
            #[doc = concat!("Decode a ", $kind, " from an already parsed JSON value, moving its fields.")]
            ///
            /// Same validation as `serde_json::from_value`, which would first rebuild
            /// every map and array of `value` through the `Value` deserializer.
            ///
            /// # Errors
            ///
            #[doc = $errors]
            pub fn from_value(value: Value) -> Result<Self, serde_json::Error> {
                Self::from_object(value)
            }
        }
    };
}

macro_rules! impl_string_number_conversions {
    ($name:ident, $deserialize_error:literal) => {
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = Value::deserialize(deserializer)?;
                Self::from_owned_value(value).ok_or_else(|| D::Error::custom($deserialize_error))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::String(value) => formatter.write_str(value),
                    Self::Number(value) => write!(formatter, "{value}"),
                }
            }
        }

        impl From<std::string::String> for $name {
            fn from(value: std::string::String) -> Self {
                Self::String(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::String(value.to_string())
            }
        }

        impl From<i64> for $name {
            fn from(value: i64) -> Self {
                Self::Number(value.into())
            }
        }

        impl From<$name> for Value {
            fn from(value: $name) -> Self {
                match value {
                    $name::String(value) => Self::String(value),
                    $name::Number(value) => Self::Number(value),
                }
            }
        }
    };
}

/// A JSON-RPC request identifier.
///
/// MCP request identifier represented as a string or JSON number.
#[derive(Debug, Clone)]
pub enum RequestId {
    /// Numeric ID
    Number(serde_json::Number),
    /// String ID
    String(std::string::String),
}

impl RequestId {
    /// Whether a borrowed JSON value is a valid request ID, without copying it.
    pub fn is_valid_value(value: &Value) -> bool {
        is_valid_request_id(value)
    }

    /// Create a numeric request ID
    pub fn number(n: i64) -> Self {
        Self::Number(n.into())
    }

    /// Create a string request ID
    pub fn string(s: impl Into<std::string::String>) -> Self {
        Self::String(s.into())
    }

    /// Convert to a `serde_json::Value`
    pub fn to_value(&self) -> Value {
        match self {
            Self::Number(n) => Value::Number(n.clone()),
            Self::String(s) => Value::String(s.clone()),
        }
    }

    /// Try to convert from a `serde_json::Value`
    pub fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Number(n) if n.is_i64() || n.is_u64() => Some(Self::Number(n.clone())),
            Value::String(s) => Some(Self::String(s.clone())),
            _ => None,
        }
    }

    fn from_owned_value(value: Value) -> Option<Self> {
        match value {
            Value::Number(n) if n.is_i64() || n.is_u64() => Some(Self::Number(n)),
            Value::String(s) => Some(Self::String(s)),
            _ => None,
        }
    }
}

impl Serialize for RequestId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Number(value) if value.is_i64() || value.is_u64() => value.serialize(serializer),
            Self::Number(_) => Err(S::Error::custom("request id number must be an integer")),
            Self::String(value) => value.serialize(serializer),
        }
    }
}

impl_string_number_conversions!(RequestId, "request id must be a string or integer");

impl PartialEq for RequestId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for RequestId {}

impl Hash for RequestId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Number(n) => n.hash(state),
            Self::String(s) => s.hash(state),
        }
    }
}

/// Opaque token used to correlate progress notifications with a request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProgressToken {
    /// String token.
    String(std::string::String),
    /// Numeric token.
    Number(serde_json::Number),
}

impl ProgressToken {
    /// Convert the token to its JSON representation.
    pub fn to_value(&self) -> Value {
        match self {
            Self::String(value) => Value::String(value.clone()),
            Self::Number(value) => Value::Number(value.clone()),
        }
    }

    /// Convert a valid JSON string or number to a progress token.
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::String(value) => Some(Self::String(value.clone())),
            Value::Number(value) => Some(Self::Number(value.clone())),
            _ => None,
        }
    }

    fn from_owned_value(value: Value) -> Option<Self> {
        match value {
            Value::String(value) => Some(Self::String(value)),
            Value::Number(value) => Some(Self::Number(value)),
            _ => None,
        }
    }
}

impl Serialize for ProgressToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::String(value) => value.serialize(serializer),
            Self::Number(value) => value.serialize(serializer),
        }
    }
}

impl_string_number_conversions!(ProgressToken, "progress token must be a string or number");

/// The JSON-RPC protocol version, `"2.0"`.
///
/// Zero-sized: it serializes as the literal `"2.0"` without storing or
/// allocating a string, and deserialization rejects every other value, so an
/// envelope with an invalid version cannot be constructed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct JsonRpcVersion;

impl JsonRpcVersion {
    /// The only supported version string.
    pub const VALUE: &'static str = crate::protocol::version::JSON_RPC_VERSION;

    /// The version string, `"2.0"`.
    pub const fn as_str(self) -> &'static str {
        Self::VALUE
    }

    fn parse<E: serde::de::Error>(version: &str) -> Result<Self, E> {
        if version == Self::VALUE {
            Ok(Self)
        } else {
            Err(E::custom(VERSION_ERROR))
        }
    }
}

impl fmt::Display for JsonRpcVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(Self::VALUE)
    }
}

impl Serialize for JsonRpcVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(Self::VALUE)
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct VersionVisitor;

        impl Visitor<'_> for VersionVisitor {
            type Value = JsonRpcVersion;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("the JSON-RPC version string \"2.0\"")
            }

            fn visit_str<E: serde::de::Error>(self, version: &str) -> Result<JsonRpcVersion, E> {
                JsonRpcVersion::parse(version)
            }
        }

        deserializer.deserialize_str(VersionVisitor)
    }
}

/// A JSON-RPC 2.0 message as received on the wire.
///
/// Distinguishes between requests (has `method` + `id`), notifications
/// (has `method`, no `id`), and responses (has `result` or `error` + `id`).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    /// A request with an ID expecting a response
    Request(JsonRpcRequest),
    /// A notification (request with no ID)
    Notification(JsonRpcRequest),
    /// A response to a previous request
    Response(JsonRpcResponse),
}

impl<'de> Deserialize<'de> for JsonRpcMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Value = Value::deserialize(deserializer)?;

        let obj = raw
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("JSON-RPC message must be an object"))?;

        if obj.contains_key("method") {
            let request = JsonRpcRequest::from_object::<D::Error>(raw)
                .map_err(|e| serde::de::Error::custom(format!("invalid request: {e}")))?;
            if request.id.is_some() {
                Ok(Self::Request(request))
            } else {
                Ok(Self::Notification(request))
            }
        } else if obj.contains_key("result") || obj.contains_key("error") {
            let response = JsonRpcResponse::from_object::<D::Error>(raw)
                .map_err(|e| serde::de::Error::custom(format!("invalid response: {e}")))?;
            Ok(Self::Response(response))
        } else {
            Err(serde::de::Error::custom(
                "JSON-RPC message must have 'method' (request/notification) or 'result'/'error' (response)",
            ))
        }
    }
}

impl JsonRpcMessage {
    /// Returns the request ID if this is a request or response (not a notification)
    pub const fn id(&self) -> Option<&Value> {
        match self {
            Self::Request(r) => r.id.as_ref(),
            Self::Response(r) => r.id.as_ref(),
            Self::Notification(_) => None,
        }
    }

    /// Returns the method name if this is a request or notification
    pub fn method(&self) -> Option<&str> {
        match self {
            Self::Request(r) | Self::Notification(r) => Some(&r.method),
            Self::Response(_) => None,
        }
    }
}

/// JSON-RPC 2.0 request
#[derive(Debug, Clone)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: JsonRpcVersion,
    /// Request ID (omitted for notifications)
    pub id: Option<Value>,
    /// Method name
    pub method: String,
    /// Method parameters
    pub params: Option<Value>,
}

impl Serialize for JsonRpcRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(id) = &self.id
            && !is_valid_request_id(id)
        {
            return Err(S::Error::custom(
                "JSON-RPC request id must be a string or integer",
            ));
        }
        if let Some(params) = &self.params
            && !params.is_object()
        {
            return Err(S::Error::custom(
                "JSON-RPC request params must be an object",
            ));
        }

        let mut state = serializer.serialize_struct(
            "JsonRpcRequest",
            2 + usize::from(self.id.is_some()) + usize::from(self.params.is_some()),
        )?;
        state.serialize_field("jsonrpc", &self.jsonrpc)?;
        if let Some(id) = &self.id {
            state.serialize_field("id", id)?;
        }
        state.serialize_field("method", &self.method)?;
        if let Some(params) = &self.params {
            state.serialize_field("params", params)?;
        }
        state.end()
    }
}

owned_value_decoding!(
    JsonRpcRequest,
    "request",
    "Fails when `value` is not an object, `jsonrpc` is not `\"2.0\"`, it carries \
     `result` or `error`, `id` is neither a string nor an integer, `method` is \
     missing or not a string, or `params` is present but not an object."
);

impl JsonRpcRequest {
    fn from_object<E: serde::de::Error>(value: Value) -> Result<Self, E> {
        let Value::Object(mut object) = value else {
            return Err(E::custom("JSON-RPC request must be an object"));
        };

        let jsonrpc = required_version::<E>(&mut object)?;
        if object.contains_key("result") || object.contains_key("error") {
            return Err(E::custom(
                "JSON-RPC request must not contain result or error",
            ));
        }

        let id = object.remove("id").map(|id| {
            if !is_valid_request_id(&id) {
                return Err(E::custom("JSON-RPC request id must be a string or integer"));
            }
            Ok(id)
        });
        let id = id.transpose()?;
        let method = required_string::<E>(&mut object, "method")?;
        let params = match object.remove("params") {
            Some(value) if value.is_object() => Some(value),
            Some(_) => return Err(E::custom("JSON-RPC request params must be an object")),
            None => None,
        };

        Ok(Self {
            jsonrpc,
            id,
            method,
            params,
        })
    }

    /// Create a new request with ID
    pub fn new(id: Value, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JsonRpcVersion,
            id: Some(id),
            method: method.into(),
            params,
        }
    }

    /// Create a notification (no ID)
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JsonRpcVersion,
            id: None,
            method: method.into(),
            params,
        }
    }

    /// Check if this is a notification
    pub const fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: JsonRpcVersion,
    /// Request ID
    pub id: Option<Value>,
    /// Successful result
    pub result: Option<Value>,
    /// Error object
    pub error: Option<JsonRpcError>,
}

impl Serialize for JsonRpcResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match (&self.result, &self.error) {
            (Some(_), None) | (None, Some(_)) => {}
            (Some(_), Some(_)) => {
                return Err(S::Error::custom(
                    "JSON-RPC response must not contain both result and error",
                ));
            }
            (None, None) => {
                return Err(S::Error::custom(
                    "JSON-RPC response must contain result or error",
                ));
            }
        }

        if self.result.is_some() && !self.id.as_ref().is_some_and(is_valid_request_id) {
            return Err(S::Error::custom(
                "successful JSON-RPC response id must be a string or integer",
            ));
        }
        if self
            .result
            .as_ref()
            .is_some_and(|result| !result.is_object())
        {
            return Err(S::Error::custom(
                "successful MCP response result must be an object",
            ));
        }
        if self.error.is_some() && self.id.as_ref().is_some_and(|id| !is_valid_request_id(id)) {
            return Err(S::Error::custom(
                "JSON-RPC error response id must be omitted, a string, or an integer",
            ));
        }

        let mut state = serializer.serialize_struct("JsonRpcResponse", 3)?;
        state.serialize_field("jsonrpc", &self.jsonrpc)?;
        if let Some(id) = &self.id {
            state.serialize_field("id", id)?;
        }
        if let Some(result) = &self.result {
            state.serialize_field("result", result)?;
        } else if let Some(error) = &self.error {
            state.serialize_field("error", error)?;
        }
        state.end()
    }
}

owned_value_decoding!(
    JsonRpcResponse,
    "response",
    "Fails when `value` is not an object, `jsonrpc` is not `\"2.0\"`, it carries \
     `method`, it does not hold exactly one of `result` and `error`, its `id` is \
     not a string or integer (an error response may omit it), `result` is not \
     an object, or `error` is not a valid error object."
);

impl JsonRpcResponse {
    fn from_object<E: serde::de::Error>(value: Value) -> Result<Self, E> {
        let Value::Object(mut object) = value else {
            return Err(E::custom("JSON-RPC response must be an object"));
        };

        let jsonrpc = required_version::<E>(&mut object)?;
        if object.contains_key("method") {
            return Err(E::custom("JSON-RPC response must not contain method"));
        }

        let has_result = object.contains_key("result");
        let has_error = object.contains_key("error");
        if has_result == has_error {
            return Err(E::custom(
                "JSON-RPC response must contain exactly one of result or error",
            ));
        }

        let id = object.remove("id");
        if (has_result && !id.as_ref().is_some_and(is_valid_request_id))
            || (has_error && id.as_ref().is_some_and(|id| !is_valid_request_id(id)))
        {
            return Err(E::custom(if has_result {
                "successful JSON-RPC response id must be a string or integer"
            } else {
                "JSON-RPC error response id must be omitted, a string, or an integer"
            }));
        }

        let result = object.remove("result");
        if result.as_ref().is_some_and(|result| !result.is_object()) {
            return Err(E::custom(
                "successful MCP response result must be an object",
            ));
        }
        let error = object
            .remove("error")
            .map(|error| JsonRpcError::from_owned_value(error).map_err(E::custom))
            .transpose()?;

        Ok(Self {
            jsonrpc,
            id,
            result,
            error,
        })
    }
}

fn required_string<E>(
    object: &mut serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<String, E>
where
    E: serde::de::Error,
{
    match object
        .remove(field)
        .ok_or_else(|| E::missing_field(field))?
    {
        Value::String(value) => Ok(value),
        _ => Err(E::custom(format!("JSON-RPC {field} must be a string"))),
    }
}

fn required_version<E>(object: &mut serde_json::Map<String, Value>) -> Result<JsonRpcVersion, E>
where
    E: serde::de::Error,
{
    JsonRpcVersion::parse(&required_string::<E>(object, "jsonrpc")?)
}

fn is_valid_request_id(id: &Value) -> bool {
    match id {
        Value::String(_) => true,
        Value::Number(number) => number.is_i64() || number.is_u64(),
        _ => false,
    }
}

impl JsonRpcResponse {
    /// Create a successful response
    pub const fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JsonRpcVersion,
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub const fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JsonRpcVersion,
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC 2.0 error codes
pub struct ErrorCode;

impl ErrorCode {
    /// Parse error (-32700)
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid request (-32600)
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found (-32601)
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params (-32602)
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error (-32603)
    pub const INTERNAL_ERROR: i32 = -32603;
    /// Generic server error (-32000)
    pub const SERVER_ERROR: i32 = -32000;
    /// Unauthorized request (-32001)
    pub const UNAUTHORIZED: i32 = -32001;
    /// Resource not found (-32002)
    pub const RESOURCE_NOT_FOUND: i32 = -32002;
}

impl JsonRpcError {
    /// Decode an error object, moving `data` where `serde_json::from_value`
    /// would rebuild it; same validation and messages.
    fn from_owned_value(value: Value) -> Result<Self, serde_json::Error> {
        let Value::Object(mut object) = value else {
            return serde_json::from_value(value);
        };
        // Fields in key order, so the first error matches serde's.
        let code = object.remove("code").map(i32::deserialize).transpose()?;
        let data = object.remove("data").filter(|data| !data.is_null());
        let message = object
            .remove("message")
            .map(String::deserialize)
            .transpose()?;
        Ok(Self {
            code: code.ok_or_else(|| serde_json::Error::missing_field("code"))?,
            message: message.ok_or_else(|| serde_json::Error::missing_field("message"))?,
            data,
        })
    }

    /// Create a new error
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a new error with additional data
    pub fn new_with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    fluent_setter!(
        /// Add additional data to the error
        pub fn with_data(data: Value) => some data
    );

    jsonrpc_error_constructors! {
        /// Parse error (-32700)
        parse_error => PARSE_ERROR;
        /// Invalid request (-32600)
        invalid_request => INVALID_REQUEST;
        /// Method not found (-32601)
        method_not_found => METHOD_NOT_FOUND;
        /// Invalid params (-32602)
        invalid_params => INVALID_PARAMS;
        /// Internal error (-32603)
        internal_error => INTERNAL_ERROR;
    }
}
