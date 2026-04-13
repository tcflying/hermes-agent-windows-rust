pub mod auxiliary_client;
pub mod chat;
pub mod compression;
pub mod error_classifier;
pub mod interrupt;
pub mod iteration;
pub mod memory;
pub mod memory_nudge;
pub mod prompt_builder;
pub mod retry_utils;
pub mod skill_commands;
pub mod tools;

pub use chat::{run_conversation, ChatResponse, Message, ToolCall, get_tool_definitions};
pub use iteration::IterationBudget;
pub use interrupt::InterruptFlag;
pub use memory::{MemoryStore, MemoryManager, MemorySnapshot};
pub use memory_nudge::{MemoryNudge, NudgeInjector, NudgeConfig};
pub use prompt_builder::PromptBuilder;
pub use error_classifier::{classify_error, is_retryable, ClassifiedError, ErrorCategory};
pub use retry_utils::{RetryConfig, retry_with_backoff, retry_api_call, calculate_delay};
pub use auxiliary_client::AuxiliaryClient;
pub use tools::skill_manager::SkillManager;


