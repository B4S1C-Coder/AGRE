mod role;
mod message;
mod tool_call;

pub use role::Role;
pub use message::Message;
pub use tool_call::ToolCall;

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