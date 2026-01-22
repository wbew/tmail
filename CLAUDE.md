# Claude Instructions

Keep entries succinct and simple.

## CLI Development

After completing a CLI feature, show:
- Example usage
- A `cargo run` command for easy testing

**Never test using the CLI directly** - it affects the user's real account.

## Testing

Use integration tests in `src/lib.rs` to verify new functionality:
```bash
cargo test -- --ignored
```
