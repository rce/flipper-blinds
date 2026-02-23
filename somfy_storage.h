#pragma once

#include "somfy_rts.h"

// Load blind state from persistent storage. Initializes empty state if file missing.
void somfy_state_load(SomfyState* state);

// Save blind state to persistent storage.
bool somfy_state_save(const SomfyState* state);
