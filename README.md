<div align="center">

# Markdown Extract

[![Crates.io](https://img.shields.io/crates/v/markdown-extract)](https://crates.io/crates/markdown-extract)
[![Docker Pulls](https://img.shields.io/docker/pulls/sean0x42/markdown-extract)](https://hub.docker.com/r/sean0x42/markdown-extract)
[![Build & Test](https://github.com/sean0x42/markdown-extract/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/sean0x42/markdown-extract/actions/workflows/build_and_test.yml)

</div>

Extract sections of a markdown file with a regular expression.

## Usage

Given a document called `my-document.md`:

```markdown
# Welcome

This is my amazing markdown document.

## Extract me!

This section should be pulled out.
```

You can extract the second section with the following command:

```console
$ markdown-extract "Extract me!" my-document.md
## Extract me!

This section should be pulled out.
```

`markdown-extract` plays nicely in pipelines: if you stream output into a
command that closes the pipe early (for example `head`), the CLI will stop
writing quietly instead of printing broken pipe errors.

## Installation

If you've got Rust installed on your system, you can simply install
`markdown-extract` with Cargo.

```console
$ cargo install markdown-extract-cli
```

### Docker

A Docker image is also available, and can be installed with the following
command:

```console
$ docker pull sean0x42/markdown-extract:v2
```

You can then run the container with the following command:

```console
$ docker run -it sean0x42/markdown-extract:v2 --help
```

Note that because markdown-extract accesses the file system, you will need
to mount a volume if you want to access a file on the host. e.g.

``` console
$ docker run -v $PWD:/opt -it sean0x42/markdown-extract:v2 v2.0.0 /opt/CHANGELOG.md
```

If you know a better way of achieving this, please let me know!

## Github Action

This project can be used as a Github action.

Here is a sample workflow usage:

```yaml
- id: extract-changelog
  uses: sean0x42/markdown-extract@v4
  with:
   file: crates/markdown-extract/CHANGELOG.md
   pattern: 'v2.0.0'
- name: Write output to file
  run: |
    printf '${{ steps.extract-changelog.outputs.markdown }}' > CHANGELOG-extracted.txt
- uses: actions/upload-artifact@v3
  with:
   name: changelog
   path: CHANGELOG-extracted.txt
```

For a complete reference view the [action.yml](action.yml).

The action version corresponds to the version of the tool.

## Use Cases

There aren't many, to be honest.

1. Extract patch notes from a `CHANGELOG.md` by version.
2. The talented folks at HashiCorp are using `markdown-extract` to extract API
   documentation, and inject it into OpenAPI schemas.

If you have another use for this tool, please let me know!

## AI Agent Tooling

LLM-based agents tend to work best when you keep their context windows lean. Pairing `markdown-extract` with your prompt assembly scripts lets you grab only the sections agents need—no more, no less.

- Pull targeted instructions or API notes into an agent's scratch space before a run.
- Pre-filter long knowledge bases so retrieval pipelines send lightweight Markdown snippets to the model.
- Combine with schedulers (like GitHub Actions or cron) to refresh agent-ready context automatically from evolving docs.

Because the CLI writes clean Markdown to stdout, it slips neatly into shell pipelines or any orchestration layer that can run a process and ingest its output—making it a low-maintenance companion for automated agent workflows.
