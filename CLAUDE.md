# dewiz-stealth-address

## Project Overview

Solidity smart contract project implementing stealth address functionality.

## Tech Stack

- **Language:** Solidity
- **Framework:** Foundry (`forge`, `cast`, `anvil`)
- **Testing:** Forge tests

## Common Commands

```bash
forge build        # Compile contracts
forge test         # Run all tests
forge test -vvv    # Run tests with verbose output
forge fmt          # Format Solidity code
forge snapshot     # Generate gas snapshots
```

## Repository Structure

- `src/` — Smart contract source files
- `test/` — Foundry test files
- `script/` — Deployment and utility scripts
- `lib/` — Git submodule dependencies

## Code Conventions

- Follow the Sky Protocol Solidity style guide
- Use NatSpec comments for all public and external functions
- Use custom errors instead of revert strings
- All state-changing functions must emit events
