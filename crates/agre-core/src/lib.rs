mod message;
mod role;
mod tool_call;
mod tool_definition;
mod tool_result;

pub use message::Message;
pub use role::Role;
pub use tool_call::ToolCall;
pub use tool_definition::{
  FunctionDefinition, ParameterSchema, SchemaType, ToolDefinition, ToolSchema, ToolType,
};
pub use tool_result::ToolResult;

/*
| Trait         | What it gives you                                                            | Example                                 |
| ------------- | ---------------------------------------------------------------------------- | --------------------------------------- |
| `Debug`       | Lets you print the value for debugging                                       | `println!("{:?}", msg)`                 |
| `Clone`       | Lets you explicitly make a copy of the value                                 | `let b = a.clone()`                     |
| `Serialize`   | Converts the Rust value → serialized format like JSON                        | `serde_json::to_string(&msg)`           |
| `Deserialize` | Converts JSON/etc. → Rust value                                              | `serde_json::from_str::<Message>(json)` |
| `PartialEq`   | Allows equality comparison with `==` / `!=`                                  | `a == b`                                |
| `Eq`          | Says equality is a proper equivalence relation; stronger form of `PartialEq` | Used by things like some collections    |
*/

/*
LLM response
    |
    | arguments = "{\"expression\":\"2 + 2\"}"
    v
ToolCall.arguments: String
    |
    v
runtime parses it
    |
    +---- valid JSON ----> tool
    |
    +---- invalid JSON --> repair observation
*/
