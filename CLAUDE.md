# Summary

We are building an open source social media tool that aggregates posts from the top tier open source social media networks: Hashiverse, nostr, Mastodon, and Bluesky.

The tool is a SPA (single page application) written in Rust/WASM (for the heavy lifting), and React/Mantine (for the GUI).  There is no server (other than the hosting of the SPA) - everything lives in the browser.

# Specification

Part of your role is to ensure that a detailed spec of the product is maintained in /specs.  This folder will be a hierarchy of folders containing markdown files that describe the specification.  This hierarchy is designed to allow AI agents to efficiently navigate subsections of the spec without filling its context with the entire spec.

An index.md will describe the specification covered in each subfolder, and will link to other markdown files in that subfolder.  If ever any individual markdown file gets too long, it will be replaced with a subfolder of the same name and the too-long markdown file will be splintered into its own index.md and sub-markdown files inside that child subfolder.

## Repository Layout

```
/specs                                        # The hierarchical specs folder
/muxsocial-client-web                         # The Typescript/React/Mantine GUI consuming muxsocial-client-wasm
/muxsocial-rust                               # Rust workspace
/muxsocial-rust/muxsocial-lib                 # Rust library containing the bulk of functionality
/muxsocial-rust/muxsocial-client-wasm         # Rust wrapper exposing some of the muxsocial-lib to WASM/Typescript
/muxsocial-rust/muxsocial-integration-tests   # Long-running tests that are broader than unit tests, also a TUI for interactive tests
```

## Top level directives

- Be direct and not obseqious.  I don't need flattery.
- Prefer long variables names like are already in the codebase - generally prefer `encoded_post_bundle_feedback: EncodedPostBundleFeedbackV1` and `let bytes_gatherer: BytesGatherer = xxx` over `let g: BytesGatherer = xxx` 
- In git, omit "Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>" from commit messages
- Every suggested addition or refactor should be able to be tested using tests - write them if they are missing.  For a refactor, lets write any missing tests and test them before doing the refactor.
- All strings in both rust and typescript should be " delimited, not '
