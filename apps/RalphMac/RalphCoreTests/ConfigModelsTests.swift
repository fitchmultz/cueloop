/**
 ConfigModelsTests

 Purpose:
 - Regression-test decoding of RalphCore config models against CLI-shaped JSON payloads.

 Responsibilities:
 - Regression-test decoding of RalphCore config models against CLI-shaped JSON payloads.

 Does not handle:
 - Config validation semantics (CLI remains source of truth).

 Usage:
 - Used by the RalphMac app or RalphCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Fixtures mirror snake_case keys from `ralph machine config resolve` documents.
 */

import Foundation
import XCTest

@testable import RalphCore

final class ConfigModelsTests: RalphCoreTestCase {
    func test_decode_machineConfigResolve_includesWebhookUrlPolicyFields() throws {
        let json = #"""
        {
          "version": 4,
          "paths": {
            "repo_root": "/tmp/ws",
            "queue_path": "/tmp/ws/.ralph/queue.jsonc",
            "done_path": "/tmp/ws/.ralph/done.jsonc",
            "project_config_path": "/tmp/ws/.ralph/config.jsonc",
            "global_config_path": null
          },
          "safety": {
            "repo_trusted": true,
            "dirty_repo": false,
            "git_publish_mode": "off",
            "approval_mode": "default",
            "ci_gate_enabled": true,
            "git_revert_mode": "ask",
            "parallel_configured": false,
            "execution_interactivity": "noninteractive_streaming",
            "interactive_approval_supported": false
          },
          "config": {
            "agent": {
              "runner": "codex",
              "model": "gpt-5.4",
              "webhook": {
                "enabled": true,
                "url": "https://hooks.example.com/ralph",
                "allow_insecure_http": true,
                "allow_private_targets": true,
                "retry_count": 5,
                "retry_backoff_ms": 2000,
                "secret": "redacted",
                "timeout_secs": 30
              }
            }
          },
          "execution_controls": {
            "runners": [
              {
                "id": "codex",
                "display_name": "OpenAI Codex CLI",
                "source": "built_in",
                "reasoning_effort_supported": true,
                "supports_arbitrary_model": false,
                "allowed_models": ["gpt-5.4"],
                "default_model": "gpt-5.4"
              },
              {
                "id": "acme.runner",
                "display_name": "Acme Runner",
                "source": "project_plugin",
                "reasoning_effort_supported": false,
                "supports_arbitrary_model": true,
                "default_model": "acme-fast"
              }
            ],
            "reasoning_efforts": ["low", "medium", "high", "xhigh"],
            "parallel_workers": {
              "min": 2,
              "max": 255,
              "default_missing_value": 2
            }
          },
          "resume_preview": null
        }
        """#

        let doc = try JSONDecoder().decode(MachineConfigResolveDocument.self, from: Data(json.utf8))
        XCTAssertEqual(doc.version, 4)
        let webhook = try XCTUnwrap(doc.config.agent?.webhook)
        XCTAssertEqual(webhook.enabled, true)
        XCTAssertEqual(webhook.url, "https://hooks.example.com/ralph")
        XCTAssertEqual(webhook.allowInsecureHttp, true)
        XCTAssertEqual(webhook.allowPrivateTargets, true)
        XCTAssertEqual(webhook.retryCount, 5)
        XCTAssertEqual(webhook.retryBackoffMs, 2000)
        XCTAssertEqual(doc.executionControls.runners.map(\.id), ["codex", "acme.runner"])
        XCTAssertEqual(doc.executionControls.reasoningEfforts, ["low", "medium", "high", "xhigh"])
        XCTAssertEqual(doc.executionControls.parallelWorkers.max, 255)
    }

    func test_decode_ralphConfig_notification_includesWatchNewTasksField() throws {
        let json = #"""
        {
          "agent": {
            "notification": {
              "notify_on_watch_new_tasks": false
            }
          }
        }
        """#

        let config = try JSONDecoder().decode(RalphConfig.self, from: Data(json.utf8))
        let notification = try XCTUnwrap(config.agent?.notification)
        XCTAssertEqual(notification.notifyOnWatchNewTasks, false)
    }
}
