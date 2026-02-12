# Contributing

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for automatic versioning.

### Format

```
<type>: <description>

[optional body]
```

### Types

| Type | Version Bump | Description |
|------|--------------|-------------|
| `feat` | Minor | New feature |
| `fix` | Patch | Bug fix |
| `docs` | - | Documentation only |
| `chore` | - | Maintenance tasks |
| `refactor` | - | Code refactoring |
| `test` | - | Adding tests |

### Breaking Changes

Add `!` after the type or include `BREAKING CHANGE:` in the body for major version bumps:

```
feat!: remove deprecated API

BREAKING CHANGE: The old API has been removed.
```

### Examples

```
feat: add beatmap preview on hover
fix: resolve connection timeout on slow networks
docs: update installation instructions
chore: update dependencies
```
