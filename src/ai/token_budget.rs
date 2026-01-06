pub struct TokenBudget {
    pub max_tokens: usize,
    pub current: usize,
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            current: 0,
        }
    }

    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.current)
    }

    pub fn can_afford(&self, estimated_tokens: usize) -> bool {
        self.current + estimated_tokens <= self.max_tokens
    }

    pub fn consume(&mut self, tokens: usize) {
        self.current += tokens;
    }

    pub fn reset(&mut self) {
        self.current = 0;
    }

    /// Estimate token count for a string.
    /// A common rule of thumb is 1 token ~ 4 characters.
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        // Basic heuristic: 1 token approx 4 chars
        text.len().div_ceil(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_management() {
        let mut budget = TokenBudget::new(100);
        assert_eq!(budget.remaining(), 100);

        budget.consume(20);
        assert_eq!(budget.remaining(), 80);
        assert_eq!(budget.current, 20);

        assert!(budget.can_afford(10));
        assert!(!budget.can_afford(90));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(TokenBudget::estimate_tokens(""), 0);
        assert_eq!(TokenBudget::estimate_tokens("1234"), 1);
        assert_eq!(TokenBudget::estimate_tokens("12345"), 2);
    }
}
