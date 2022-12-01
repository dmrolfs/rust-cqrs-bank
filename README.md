# Rust-CQRS-EventSourcing

## Information

## Introduction

## Getting Started

### Setup Rust toolchain

The **strongly** recommended path to install Rust on your system is through `rustup`.

Instructions on how to install `rustup` itself can be found at `https://rustup.rs`.

`rustup` is more than a Rust installer -- it Rust's *toolchain management* system providing easy access to maintain the versions of core Rust tools, beyond the compiler, `rustc`. `rustup" also manages the release channel you subscribe to. 
Although it is *highly* recommended to develop on the `stable` channel, it can be useful to temporarily run on the `nightly` channel for certain tools. `rustup` and `cargo` make that easy.

The other key tool, included in the toolchain, is `cargo`. `cargo` is the *package manager* for Rust. `cargo` manages your project dependencies, via the project's `Cargo.toml` file, and offers project lifecycle commands you would expect, such as `build`, `test`, `run`. It's like the maven or sbt for Rust and has the advantage of learning lessons from those earlier tools.

### Setup faste linking

To speed up the linking phase you have to install an alternative linker on your machine, corresponding to configuration specified in `Cargo.toml`:

#### On Windows
```shell
cargo install -f cargo-binutils
rustup component add llvm-tools-preview
```

#### On Linux:

- Ubuntu
```shell
sudo apt-get install lld clang
```

- Arch,
```shell
sudo pacman -S lld clang
```

#### On MacOS
```shell
brew install michaeleisel/zld/zld
```

## How to build the project

The project can be built with the following command:

```shell
cargo build
```
this will by default build the project in Debug mode, to build in release mode
```shell
cargo build --release
```

During development, you don't need to fully build the binary and can perform a compilation check, which is much quicker:

```shell
cargo check
```
or watch and check after file changes:
```shell
cargo watch
```

If no errors, this will generate the `bankaccount` server binary in `target/[debug/release]/bankaccount`. This is a normal executable and can be run in the command line. (For a Windows target, bankaccount.exe is produced.)

```shell
target/debug/bankaccount
```

## Running the server using `cargo`

During development, you may also run the debug or release versions via `cargo`. 
```shell
cargo run
```

```shell
cargo run --release
```

Environment variables can be provided for execution before the `cargo` command. Also, command-line arguments may be provided after `--`; e.g.,

```shell
RUST_LOG="debug" APP_ENVIRONMENT="local" cargo run -- --secrets ./resources/secrets.yaml
```

or

```shell
cargo run -- --help
```

### Try it out

In the project directory, we'll build and the release version of the server. This will take longer to build because `cargo` will build the optimized version, stripping out debug symbols and peforming additional optimizations. This will shrink the binary size and speed execution.

Building the debug version (minus the `--release` flag) builds much, much quicker. 
From the project root directory:

```shell
cargo run --release -- --secrets ./resources/secrets.yaml
```

You'll see a bunch of log lines. The last one should say something like "... API listening on 0.0.0.0:8000 ..."

Now in another terminal, perform a health check on the server:

```shell
curl --location --request GET 'localhost:8000/api/v1/health'
```
returns HTTP 200 and `{"status":"Up"}` payload.

## How To run unit and integration tests

```shell
cargo test
```

this will build and run all the tests, including unit/integration/documentation tests.

***

# Editing this README

When you're ready to make this README your own, just edit this file and use the handy template below (or feel free to structure it however you want - this is just a starting point!). Thank you to [makeareadme.com](https://www.makeareadme.com/) for this template.

## Suggestions for a good README
Every project is different, so consider which of these sections apply to yours. The sections used in the template are suggestions for most open source projects. Also keep in mind that while a README can be too long and detailed, too long is better than too short. If you think your README is too long, consider utilizing another form of documentation rather than cutting out information.

## Name
Choose a self-explaining name for your project.

## Description
Let people know what your project can do specifically. Provide context and add a link to any reference visitors might be unfamiliar with. A list of Features or a Background subsection can also be added here. If there are alternatives to your project, this is a good place to list differentiating factors.

## Badges
On some READMEs, you may see small images that convey metadata, such as whether or not all the tests are passing for the project. You can use Shields to add some to your README. Many services also have instructions for adding a badge.

## Visuals
Depending on what you are making, it can be a good idea to include screenshots or even a video (you'll frequently see GIFs rather than actual videos). Tools like ttygif can help, but check out Asciinema for a more sophisticated method.

## Installation
Within a particular ecosystem, there may be a common way of installing things, such as using Yarn, NuGet, or Homebrew. However, consider the possibility that whoever is reading your README is a novice and would like more guidance. Listing specific steps helps remove ambiguity and gets people to using your project as quickly as possible. If it only runs in a specific context like a particular programming language version or operating system or has dependencies that have to be installed manually, also add a Requirements subsection.

## Usage
Use examples liberally, and show the expected output if you can. It's helpful to have inline the smallest example of usage that you can demonstrate, while providing links to more sophisticated examples if they are too long to reasonably include in the README.

## Support
Tell people where they can go to for help. It can be any combination of an issue tracker, a chat room, an email address, etc.

## Roadmap
If you have ideas for releases in the future, it is a good idea to list them in the README.

## Contributing
State if you are open to contributions and what your requirements are for accepting them.

For people who want to make changes to your project, it's helpful to have some documentation on how to get started. Perhaps there is a script that they should run or some environment variables that they need to set. Make these steps explicit. These instructions could also be useful to your future self.

You can also document commands to lint the code or run tests. These steps help to ensure high code quality and reduce the likelihood that the changes inadvertently break something. Having instructions for running tests is especially helpful if it requires external setup, such as starting a Selenium server for testing in a browser.

## Authors and acknowledgment
Show your appreciation to those who have contributed to the project.

## License
For open source projects, say how it is licensed.

## Project status
If you have run out of energy or time for your project, put a note at the top of the README saying that development has slowed down or stopped completely. Someone may choose to fork your project or volunteer to step in as a maintainer or owner, allowing your project to keep going. You can also make an explicit request for maintainers.
