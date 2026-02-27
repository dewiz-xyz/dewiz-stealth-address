# GitHub Copilot Instructions

## Project Overview

`dewiz-stealth-address` is a Solidity smart contract project for stealth address functionality.

## Tech Stack

- **Language:** Solidity
- **Framework:** Foundry (forge, cast, anvil)
- **Testing:** Forge tests (`forge test`)
- **Package Management:** `forge install` / git submodules

## Code Style

- Follow the [Sky Protocol Solidity style guide](https://github.com/dewiz-xyz/sky-solidity-bootstrap)
- Use NatSpec comments for all public/external functions
- Prefer `custom error` types over `require` strings
- All state-changing functions should emit events

## Repository Structure

- `src/` — Smart contract source files
- `test/` — Foundry test files
- `script/` — Deployment and utility scripts
- `lib/` — Git submodule dependencies

## Testing

Run tests with:

```bash
forge test
```

## Common Commands

```bash
forge build          # Compile contracts
forge test           # Run tests
forge test -vvv      # Run tests with verbose output
forge fmt            # Format Solidity files
forge snapshot       # Gas snapshots
```
