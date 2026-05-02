/**
 CueLoopModels

 Purpose:
 - Act as the thin facade for CueLoopCore model surfaces.

 Responsibilities:
 - Act as the thin facade for CueLoopCore model surfaces.
 - Keep the historic entrypoint lightweight while concrete model types live in dedicated files.

 Does not handle:
 - Defining the CLI spec, JSON, or task models directly.
 - Any CLI execution or workspace behavior.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Concrete model declarations live in adjacent `CueLoop*` model files within CueLoopCore.
 */

import Foundation
