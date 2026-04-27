## Summary

What this PR adds, changes, or removes. One paragraph.

## Closes

Closes #

## Type

- [ ] New tool
- [ ] New skill
- [ ] Bug fix
- [ ] Documentation
- [ ] Infrastructure (CI, scripts, templates)

## Status change

- [ ] Marks an entry as `live` in `tracking.md`
- [ ] Updates an existing `live` entry's version or description
- [ ] Adds a new entry to `tracking.md`
- [ ] No `tracking.md` change needed (CI, infra, or docs only)

## Testing

How this was tested. Be specific. For tools: action calls executed and what they returned. For skills: prompts the skill activated on and what changed in the agent's response.

## Quality gates

- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy --target wasm32-wasip2 --release -- -D warnings` clean
- [ ] `cargo clippy --tests --release -- -D warnings` clean
- [ ] `cargo test` passing
