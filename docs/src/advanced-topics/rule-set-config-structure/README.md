# Rule Set Configuration File Structure

Your `apimock-rule-set.toml` file is structured into two main parts: `prefix` and `rules`.

- The `[prefix]` table allows you to define global behaviors or conditions that apply to all rules within that specific rule set file.

- The `[[rules]]` array is where you define your individual mock rules. Each `[[rules]]` block represents one rule.
