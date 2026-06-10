//! Agent-ledger CLI argument definitions.
//!
//! Purpose:
//! - Define `cueloop agent ...`, an additive task-ledger surface for already-running agents.
//!
//! Responsibilities:
//! - Keep agent-oriented queue/task tracking commands discoverable without spawning runners.
//! - Route parsed commands to the focused implementation in `commands::agent`.
//!
//! Non-scope:
//! - Runner dispatch, phase supervision, or machine API schema ownership.
//!
//! Invariants/assumptions:
//! - Commands in this group mutate only queue/task ledger state unless explicitly documented.
//! - Human task, run, runner, and app surfaces remain separate and unchanged.

use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};

use crate::commands::agent as agent_cmd;

pub fn handle_agent(args: AgentArgs, force: bool) -> Result<()> {
    agent_cmd::handle(args, force)
}

#[derive(Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Subcommand)]
pub enum AgentCommand {
    /// Show compact queue context for an already-running agent.
    #[command(
        after_long_help = "Examples:\n  cueloop agent overview\n  cueloop agent overview --format json\n  cueloop agent overview --include-done --done-limit 5"
    )]
    Overview(AgentOverviewArgs),
    /// Print the next runnable task without starting a runner.
    #[command(
        after_long_help = "Examples:\n  cueloop agent next\n  cueloop agent next --format json\n  cueloop agent next --with-title"
    )]
    Next(AgentNextArgs),
    /// Show one task from the active queue or done archive.
    #[command(
        after_long_help = "Examples:\n  cueloop agent show CL-0001\n  cueloop agent show CL-0001 --format json"
    )]
    Show(AgentTaskReadArgs),
    /// Claim a task for the current external agent/session.
    #[command(
        after_long_help = "Examples:\n  cueloop agent claim CL-0001 --owner pi-session-123\n  cueloop agent claim CL-0001 --owner codex --ttl-minutes 120"
    )]
    Claim(AgentClaimArgs),
    /// Release a previous agent claim from a task.
    #[command(
        after_long_help = "Examples:\n  cueloop agent release CL-0001\n  cueloop agent release CL-0001 --owner pi-session-123"
    )]
    Release(AgentReleaseArgs),
    /// Mark an active task as doing and append an optional note/evidence.
    #[command(
        after_long_help = "Examples:\n  cueloop agent start CL-0001 --note 'Started in current agent session'\n  cueloop agent start CL-0001 --evidence 'Reproduced failing test'"
    )]
    Start(AgentProgressArgs),
    /// Append a durable working note to an active task.
    #[command(
        after_long_help = "Examples:\n  cueloop agent note CL-0001 'Found root cause in queue validation'"
    )]
    Note(AgentTextArgs),
    /// Append verification evidence to an active task.
    #[command(
        after_long_help = "Examples:\n  cueloop agent evidence CL-0001 'make agent-ci passed'\n  cueloop agent evidence CL-0001 'cargo test -p cueloop machine_contract_test passed'"
    )]
    Evidence(AgentTextArgs),
    /// Append a plan item to an active task.
    #[command(
        name = "plan-append",
        after_long_help = "Examples:\n  cueloop agent plan-append CL-0001 'Run targeted contract tests'"
    )]
    PlanAppend(AgentTextArgs),
    /// Emit a compact handoff packet and optionally append handoff notes.
    #[command(
        after_long_help = "Examples:\n  cueloop agent handoff CL-0001\n  cueloop agent handoff CL-0001 --next 'Run make agent-ci' --format json"
    )]
    Handoff(AgentHandoffArgs),
    /// Complete an active task with required evidence and archive it.
    #[command(aliases = ["done"], after_long_help = "Examples:\n  cueloop agent complete CL-0001 --evidence 'make agent-ci passed'\n  cueloop agent done CL-0001 --note 'No residual risks' --evidence 'cargo test passed'")]
    Complete(AgentCompleteArgs),
    /// Reject an active task and archive it.
    #[command(
        after_long_help = "Examples:\n  cueloop agent reject CL-0001 --reason 'Duplicate of CL-0002'"
    )]
    Reject(AgentRejectArgs),
    /// Validate queue and archive state.
    #[command(
        after_long_help = "Examples:\n  cueloop agent validate\n  cueloop agent validate --format json"
    )]
    Validate(AgentFormatArgs),
}

#[derive(Args)]
pub struct AgentOverviewArgs {
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
    /// Include recent done-archive tasks in the compact context.
    #[arg(long)]
    pub include_done: bool,
    /// Maximum done-archive tasks to include when --include-done is set.
    #[arg(long, default_value_t = 5)]
    pub done_limit: usize,
}

#[derive(Args)]
pub struct AgentNextArgs {
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
    #[arg(long)]
    pub with_title: bool,
}

#[derive(Args)]
pub struct AgentTaskReadArgs {
    pub task_id: String,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentClaimArgs {
    pub task_id: String,
    /// Owner/session identifier for the external agent claim.
    #[arg(long)]
    pub owner: String,
    /// Optional claim lease length in minutes.
    #[arg(long)]
    pub ttl_minutes: Option<u32>,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentReleaseArgs {
    pub task_id: String,
    /// Optional owner/session expected to hold the claim.
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentProgressArgs {
    pub task_id: String,
    #[arg(long = "note")]
    pub notes: Vec<String>,
    #[arg(long = "evidence")]
    pub evidence: Vec<String>,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentTextArgs {
    pub task_id: String,
    pub text: String,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentHandoffArgs {
    pub task_id: String,
    /// Append a handoff note before printing the packet.
    #[arg(long = "note")]
    pub notes: Vec<String>,
    /// Append an explicit next-step handoff note.
    #[arg(long)]
    pub next: Option<String>,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentCompleteArgs {
    pub task_id: String,
    /// Completion evidence. At least one --evidence value is required.
    #[arg(long = "evidence", required = true)]
    pub evidence: Vec<String>,
    /// Optional completion note.
    #[arg(long = "note")]
    pub notes: Vec<String>,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentRejectArgs {
    pub task_id: String,
    /// Rejection reason stored as a lifecycle note.
    #[arg(long)]
    pub reason: String,
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Args)]
pub struct AgentFormatArgs {
    #[arg(long, value_enum, default_value_t = AgentOutputFormat::Text)]
    pub format: AgentOutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AgentOutputFormat {
    Text,
    Json,
}
