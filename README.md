<p align="center">
  <img src="https://github.com/vmalloc/pargit/blob/develop/p-logo.png?raw=true">
</p>

# Overview

pargit is a workflow utility for Git, inspired by git-flow and git-flow-avh.

## Wait, what?

`git-flow` is a tool originally published as a follow-up to [this article from 2010](https://nvie.com/posts/a-successful-git-branching-model/), which has been very appealing for developers. `git-flow` allows you to go through the various phases of release/feature cycles with relative ease, which is great.

## Why Pargit?

Although `git-flow` is great, it is far from perfect. First, it has all sorts of usability issues, some of which addressed by the `git-flow-avh` fork project. More importantly, for some types of projects, a lot of manual labour still needs to be done to publish and manage releases. This is where Pargit steps in.

Pargit aims to be an opinionated alternative to `git-flow`, while providing better automation around the tedious parts.

## Main Features

* **Atomic Releases** - when attempting to publish a release that conflicts with the upstream repo, tools like `git-flow` fail and leave you with a half-published release. Pargit fixes that by rolling back the release in a clean way and getting rid of the temporary tag created.
* **Project-Internal Versioning Logic** - pargit includes pre-release checks aimed at minimizing pain and errors. For Rust projects, it checks `Cargo.lock` correctness, performs version bumps for you, and prompts you to choose the project being bumped in multi-crate workspaces.
* **Saner Defaults** - pargit aims to make sense, deducing parameters when possible and using sane defaults for dealing with project workflow. Unlike `git-flow`, pargit will not prompt you twice for a commit message as a part of releasing a version 🤦‍♂️

# Quickstart


## Installation

```shell
$ cargo install pargit
```
## Features

Pargit forks feature branches from the `develop` branch by default. To start a new feature:
```shell
# starts a new feature, and places you in the feature/my_feature branch
$ pargit feature start my_feature 
# deletes a feature (defaults to the current one)
$ pargit feature delete [feature name] 
# publishes a feature branch to a matching remote branch, setting its upstream (defaults to the current feature)
$ pargit feature publish [feature name]
```

## Releases

```shell
# Start a new release from the develop branch
$ pargit release start 0.1.0
# Alternatively, you can tell pargit to bump a patch, minor or major version numbers
$ pargit release start minor
# You can publish a release branch to a remote branch, setting its upstream
$ pargit release publish [release name]
# When you're done, finish the release
$ pargit release finish [release name]
```

Pargit also supports quick version releases, which does the version release steps in succession for you:
```shell
$ pargit release version 0.2.0
```
Or you can specify a major/minor/patch bump:
```shell
$ pargit release version major
```

# Configuration

You can configure pargit by adding a `.pargit.toml` file in your project's root directory, in the following format (all values optional):

```toml
tag_prefix = "" # prefix for tags, e.g. "v". Default is empty prefix