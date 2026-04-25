//! Integration test hub for the `ralph machine` contract surface.
//!
//! Purpose:
//! - Group `ralph machine` integration coverage into focused contract suites.
//!
//! Responsibilities:
//! - Keep the root machine contract entrypoint thin for clearer failure locality.
//! - Route queue, task, recovery, parallel, and system coverage into adjacent modules.
//! - Expose suite-local support helpers without duplicating the shared integration harness.
//!
//! Non-scope:
//! - Holding individual test scenarios or shared fixture logic inline in this hub.
//! - Replacing broader integration helpers already owned by `crates/ralph/tests/test_support.rs`.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions callers must respect:
//! - Behavior coverage remains equivalent to the historical flat machine contract suite.
//! - Shared helpers stay local to this suite unless promoted deliberately into global test support.

#[path = "machine_contract_test/machine_contract_test_parallel.rs"]
mod machine_contract_test_parallel;
#[path = "machine_contract_test/machine_contract_test_queue.rs"]
mod machine_contract_test_queue;
#[path = "machine_contract_test/machine_contract_test_recovery.rs"]
mod machine_contract_test_recovery;
#[path = "machine_contract_test/machine_contract_test_support.rs"]
mod machine_contract_test_support;
#[path = "machine_contract_test/machine_contract_test_system.rs"]
mod machine_contract_test_system;
#[path = "machine_contract_test/machine_contract_test_task.rs"]
mod machine_contract_test_task;
