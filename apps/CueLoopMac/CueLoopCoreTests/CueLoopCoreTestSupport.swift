/**
 CueLoopCoreTestSupport

 Purpose:
 - Act as the thin facade for CueLoopCore test support helpers split across focused files.

 Responsibilities:
 - Act as the thin facade for CueLoopCore test support helpers split across focused files.

 Does not handle:
 - Defining production behavior.
 - UI automation helpers for the separate UI-test target.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Helper implementations live in focused companion files and preserve the shared test-support API.
 */

enum CueLoopCoreTestSupport {}
