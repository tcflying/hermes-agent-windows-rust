#[derive(Clone, Debug)]
pub struct NudgeConfig {
    pub interval_turns: usize,
    pub max_nudge_per_session: usize,
    pub template: String,
}

impl Default for NudgeConfig {
    fn default() -> Self {
        Self {
            interval_turns: 8,
            max_nudge_per_session: 5,
            template: "Before continuing, consider saving any important facts, patterns, or user preferences you've learned to your memory. Use the memory tool with action 'add' to persist knowledge that would be useful in future sessions.".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MemoryNudge {
    config: NudgeConfig,
    nudge_count: usize,
    turns_since_nudge: usize,
    turns_since_memory: usize,
}

impl MemoryNudge {
    pub fn new(config: NudgeConfig) -> Self {
        Self {
            config,
            nudge_count: 0,
            turns_since_nudge: 0,
            turns_since_memory: 0,
        }
    }

    pub fn should_nudge(&self, turn_count: usize) -> bool {
        if self.nudge_count >= self.config.max_nudge_per_session {
            return false;
        }
        if self.turns_since_memory < 2 {
            return false;
        }
        turn_count > 0 && turn_count % self.config.interval_turns == 0
    }

    pub fn get_nudge_message(&self) -> String {
        self.config.template.clone()
    }

    pub fn record_nudge(&mut self) {
        self.nudge_count += 1;
        self.turns_since_nudge = 0;
    }

    pub fn record_turn(&mut self) {
        self.turns_since_nudge += 1;
        self.turns_since_memory += 1;
    }

    pub fn record_memory_activity(&mut self) {
        self.turns_since_memory = 0;
    }

    pub fn reset(&mut self) {
        self.nudge_count = 0;
        self.turns_since_nudge = 0;
        self.turns_since_memory = 0;
    }
}

impl Default for MemoryNudge {
    fn default() -> Self {
        Self::new(NudgeConfig::default())
    }
}

pub struct NudgeInjector {
    nudge: MemoryNudge,
}

impl NudgeInjector {
    pub fn new() -> Self {
        Self {
            nudge: MemoryNudge::default(),
        }
    }

    pub fn check_and_generate_nudge(
        &mut self,
        turn_count: usize,
        has_recent_memory_activity: bool,
    ) -> Option<String> {
        self.nudge.record_turn();
        if has_recent_memory_activity {
            self.nudge.record_memory_activity();
        }
        if self.nudge.should_nudge(turn_count) {
            let msg = self.nudge.get_nudge_message();
            self.nudge.record_nudge();
            Some(msg)
        } else {
            None
        }
    }

    pub fn notify_memory_activity(&mut self) {
        self.nudge.record_memory_activity();
    }

    pub fn reset(&mut self) {
        self.nudge.reset();
    }
}

impl Default for NudgeInjector {
    fn default() -> Self {
        Self::new()
    }
}
